use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use quinn::{
    crypto::rustls::QuicServerConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig,
    TransportConfig, VarInt,
};
use naia_shared::{IdentityToken, LinkConditionerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

use super::{
    conditioner::ConditionedPacketReceiver, AuthReceiver as TransportAuthReceiver,
    AuthSender as TransportAuthSender, PacketReceiver, PacketSender as TransportSender, RecvError,
    SendError, Socket as TransportSocket,
};
use crate::user::UserAuthAddr;

// Constants
const MAX_DATAGRAM_SIZE: usize = 1200;
const DATAGRAM_BUFFER_SIZE: usize = 10_000_000; // 10MB
const AUTH_TIMEOUT_SECS: u64 = 10;

// Socket
pub struct Socket {
    listen_addr: SocketAddr,
    cert_config: CertificateConfig,
    quic_config: QuicConfig,
    link_conditioner_config: Option<LinkConditionerConfig>,
}

impl Socket {
    pub fn new(addrs: &ServerAddrs, config: QuicConfig) -> Self {
        Self {
            listen_addr: addrs.quic_listen_addr,
            cert_config: config.certificate_config.clone(),
            quic_config: config,
            link_conditioner_config: None,
        }
    }

    pub fn new_with_link_conditioner(
        addrs: &ServerAddrs,
        config: QuicConfig,
        link_config: LinkConditionerConfig,
    ) -> Self {
        Self {
            listen_addr: addrs.quic_listen_addr,
            cert_config: config.certificate_config.clone(),
            quic_config: config,
            link_conditioner_config: Some(link_config),
        }
    }

    fn create_endpoint(&self) -> Result<Endpoint, String> {
        // Generate or load certificates
        let (cert_chain, private_key) = match &self.cert_config {
            CertificateConfig::SelfSigned { hostnames } => {
                generate_self_signed_cert(hostnames)
                    .map_err(|e| format!("Failed to generate self-signed cert: {}", e))?
            }
            CertificateConfig::FromBytes { cert_chain, private_key } => {
                let certs: Vec<CertificateDer> = cert_chain
                    .iter()
                    .map(|bytes| CertificateDer::from(bytes.clone()))
                    .collect();
                let key = PrivateKeyDer::try_from(private_key.clone())
                    .map_err(|_| "Invalid private key format".to_string())?;
                (certs, key)
            }
        };

        // Create server config
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| format!("Failed to create rustls config: {}", e))?;

        server_crypto.alpn_protocols = vec![b"naia-quic".to_vec()];

        let mut server_config = ServerConfig::with_crypto(Arc::new(
            QuicServerConfig::try_from(server_crypto)
                .map_err(|e| format!("Failed to create QUIC config: {}", e))?,
        ));

        // Configure transport for low latency
        let mut transport_config = TransportConfig::default();
        transport_config
            .datagram_receive_buffer_size(Some(self.quic_config.datagram_receive_buffer_size));
        transport_config.datagram_send_buffer_size(self.quic_config.datagram_send_buffer_size);
        transport_config.max_idle_timeout(Some(
            VarInt::from_u64(self.quic_config.idle_timeout.as_millis() as u64)
                .map_err(|_| "Invalid idle timeout".to_string())?
                .into(),
        ));
        transport_config.keep_alive_interval(Some(self.quic_config.keep_alive_interval));
        transport_config.initial_rtt(self.quic_config.initial_rtt);

        server_config.transport_config(Arc::new(transport_config));

        // Create endpoint
        Endpoint::server(server_config, self.listen_addr)
            .map_err(|e| format!("Failed to create QUIC endpoint: {}", e))
    }
}

impl Into<Box<dyn TransportSocket>> for Socket {
    fn into(self) -> Box<dyn TransportSocket> {
        Box::new(self)
    }
}

impl TransportSocket for Socket {
    fn listen(
        self: Box<Self>,
    ) -> (
        Box<dyn TransportAuthSender>,
        Box<dyn TransportAuthReceiver>,
        Box<dyn TransportSender>,
        Box<dyn PacketReceiver>,
    ) {
        let endpoint = self.create_endpoint().expect("Failed to create QUIC endpoint");

        // Shared state
        let connections = Arc::new(Mutex::new(HashMap::<SocketAddr, Connection>::new()));
        let auth_queue = Arc::new(Mutex::new(VecDeque::<(UserAuthAddr, Vec<u8>)>::new()));
        let datagram_buffer = Arc::new(Mutex::new(VecDeque::<(SocketAddr, Vec<u8>)>::new()));

        // Spawn connection acceptor task
        spawn_connection_acceptor(
            endpoint.clone(),
            connections.clone(),
            auth_queue.clone(),
            datagram_buffer.clone(),
        );

        let auth_sender = AuthSender::new(connections.clone());
        let auth_receiver = AuthReceiver::new(auth_queue);
        let packet_sender = QuicPacketSender::new(connections.clone());
        let packet_receiver = QuicPacketReceiver::new(datagram_buffer);

        let packet_receiver: Box<dyn PacketReceiver> = {
            if let Some(config) = &self.link_conditioner_config {
                Box::new(ConditionedPacketReceiver::new(packet_receiver, config))
            } else {
                Box::new(packet_receiver)
            }
        };

        (
            Box::new(auth_sender),
            Box::new(auth_receiver),
            Box::new(packet_sender),
            packet_receiver,
        )
    }
}

// Spawn async task to accept connections
fn spawn_connection_acceptor(
    endpoint: Endpoint,
    connections: Arc<Mutex<HashMap<SocketAddr, Connection>>>,
    auth_queue: Arc<Mutex<VecDeque<(UserAuthAddr, Vec<u8>)>>>,
    datagram_buffer: Arc<Mutex<VecDeque<(SocketAddr, Vec<u8>)>>>,
) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        runtime.block_on(async {
            loop {
                match endpoint.accept().await {
                    Some(incoming) => {
                        let connections = connections.clone();
                        let auth_queue = auth_queue.clone();
                        let datagram_buffer = datagram_buffer.clone();

                        tokio::spawn(async move {
                            if let Err(e) =
                                handle_connection(incoming, connections, auth_queue, datagram_buffer)
                                    .await
                            {
                                log::warn!("Connection handling error: {}", e);
                            }
                        });
                    }
                    None => {
                        log::info!("QUIC endpoint closed");
                        break;
                    }
                }
            }
        });
    });
}

async fn handle_connection(
    incoming: quinn::Incoming,
    connections: Arc<Mutex<HashMap<SocketAddr, Connection>>>,
    auth_queue: Arc<Mutex<VecDeque<(UserAuthAddr, Vec<u8>)>>>,
    datagram_buffer: Arc<Mutex<VecDeque<(SocketAddr, Vec<u8>)>>>,
) -> Result<(), String> {
    let connection = incoming.await.map_err(|e| format!("Connection failed: {}", e))?;
    let remote_addr = connection.remote_address();

    log::info!("New QUIC connection from {}", remote_addr);

    // Store connection
    {
        let mut conns = connections.lock().unwrap();
        conns.insert(remote_addr, connection.clone());
    }

    // Wait for auth stream
    match tokio::time::timeout(
        Duration::from_secs(AUTH_TIMEOUT_SECS),
        connection.accept_bi(),
    )
    .await
    {
        Ok(Ok((send, recv))) => {
            handle_auth_stream(remote_addr, send, recv, auth_queue).await?;
        }
        Ok(Err(e)) => {
            log::warn!("Failed to accept auth stream from {}: {}", remote_addr, e);
            cleanup_connection(&connections, remote_addr);
            return Err(format!("Auth stream error: {}", e));
        }
        Err(_) => {
            log::warn!("Auth timeout for {}", remote_addr);
            cleanup_connection(&connections, remote_addr);
            return Err("Auth timeout".to_string());
        }
    }

    // After auth, receive datagrams
    loop {
        match connection.read_datagram().await {
            Ok(data) => {
                let mut buffer = datagram_buffer.lock().unwrap();
                buffer.push_back((remote_addr, data.to_vec()));
            }
            Err(quinn::ConnectionError::ApplicationClosed(_)) => {
                log::info!("Connection closed by client: {}", remote_addr);
                break;
            }
            Err(e) => {
                log::warn!("Datagram receive error from {}: {}", remote_addr, e);
                break;
            }
        }
    }

    cleanup_connection(&connections, remote_addr);
    Ok(())
}

async fn handle_auth_stream(
    remote_addr: SocketAddr,
    _send: SendStream,
    mut recv: RecvStream,
    auth_queue: Arc<Mutex<VecDeque<(UserAuthAddr, Vec<u8>)>>>,
) -> Result<(), String> {
    // Read auth data
    let auth_data = recv
        .read_to_end(4096) // Max 4KB auth data
        .await
        .map_err(|e| format!("Failed to read auth data: {}", e))?;

    log::debug!("Received auth data from {}: {} bytes", remote_addr, auth_data.len());

    // Queue auth request for processing
    let mut queue = auth_queue.lock().unwrap();
    queue.push_back((UserAuthAddr::new(remote_addr), auth_data));

    Ok(())
}

fn cleanup_connection(connections: &Arc<Mutex<HashMap<SocketAddr, Connection>>>, addr: SocketAddr) {
    let mut conns = connections.lock().unwrap();
    conns.remove(&addr);
    log::debug!("Cleaned up connection for {}", addr);
}

// Packet Sender
struct QuicPacketSender {
    connections: Arc<Mutex<HashMap<SocketAddr, Connection>>>,
}

impl QuicPacketSender {
    pub fn new(connections: Arc<Mutex<HashMap<SocketAddr, Connection>>>) -> Self {
        Self { connections }
    }
}

impl TransportSender for QuicPacketSender {
    fn send(&self, socket_addr: &SocketAddr, payload: &[u8]) -> Result<(), SendError> {
        let conns = self.connections.lock().unwrap();
        if let Some(connection) = conns.get(socket_addr) {
            connection.send_datagram(payload.to_vec().into()).map_err(|_| SendError)?;
            Ok(())
        } else {
            Err(SendError)
        }
    }
}

// Packet Receiver
#[derive(Clone)]
pub(crate) struct QuicPacketReceiver {
    buffer: Arc<Mutex<VecDeque<(SocketAddr, Vec<u8>)>>>,
    current_packet: Option<Vec<u8>>,
}

impl QuicPacketReceiver {
    pub fn new(buffer: Arc<Mutex<VecDeque<(SocketAddr, Vec<u8>)>>>) -> Self {
        Self {
            buffer,
            current_packet: None,
        }
    }
}

impl PacketReceiver for QuicPacketReceiver {
    fn receive(&mut self) -> Result<Option<(SocketAddr, &[u8])>, RecvError> {
        let mut buffer = self.buffer.lock().unwrap();
        if let Some((addr, data)) = buffer.pop_front() {
            self.current_packet = Some(data);
            if let Some(ref packet) = self.current_packet {
                Ok(Some((addr, packet.as_slice())))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

// AuthSender
#[derive(Clone)]
pub(crate) struct AuthSender {
    connections: Arc<Mutex<HashMap<SocketAddr, Connection>>>,
}

impl AuthSender {
    pub fn new(connections: Arc<Mutex<HashMap<SocketAddr, Connection>>>) -> Self {
        Self { connections }
    }
}

impl TransportAuthSender for AuthSender {
    fn accept(
        &self,
        address: &UserAuthAddr,
        identity_token: &IdentityToken,
    ) -> Result<(), SendError> {
        let conns = self.connections.lock().unwrap();
        if let Some(connection) = conns.get(&address.addr()) {
            let token_bytes = identity_token.to_string().into_bytes();

            // Send identity token via a unidirectional stream
            let connection = connection.clone();
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create tokio runtime");

                runtime.block_on(async {
                    match connection.open_uni().await {
                        Ok(mut send) => {
                            if let Err(e) = send.write_all(&token_bytes).await {
                                log::warn!("Failed to send auth response: {}", e);
                            }
                            let _ = send.finish();
                        }
                        Err(e) => {
                            log::warn!("Failed to open auth response stream: {}", e);
                        }
                    }
                });
            });

            Ok(())
        } else {
            Err(SendError)
        }
    }

    fn reject(&self, address: &UserAuthAddr) -> Result<(), SendError> {
        let mut conns = self.connections.lock().unwrap();
        if let Some(connection) = conns.remove(&address.addr()) {
            connection.close(VarInt::from_u32(401), b"Unauthorized");
            Ok(())
        } else {
            Err(SendError)
        }
    }
}

// AuthReceiver
#[derive(Clone)]
pub(crate) struct AuthReceiver {
    queue: Arc<Mutex<VecDeque<(UserAuthAddr, Vec<u8>)>>>,
    current_auth: Option<Vec<u8>>,
}

impl AuthReceiver {
    pub fn new(queue: Arc<Mutex<VecDeque<(UserAuthAddr, Vec<u8>)>>>) -> Self {
        Self {
            queue,
            current_auth: None,
        }
    }
}

impl TransportAuthReceiver for AuthReceiver {
    fn receive(&mut self) -> Result<Option<(UserAuthAddr, &[u8])>, RecvError> {
        let mut queue = self.queue.lock().unwrap();
        if let Some((addr, data)) = queue.pop_front() {
            self.current_auth = Some(data);
            if let Some(ref auth_data) = self.current_auth {
                Ok(Some((addr, auth_data.as_slice())))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

// Server Addresses
#[derive(Clone)]
pub struct ServerAddrs {
    /// IP Address to listen on for QUIC connections
    pub quic_listen_addr: SocketAddr,
}

impl ServerAddrs {
    pub fn new(quic_listen_addr: SocketAddr) -> Self {
        Self { quic_listen_addr }
    }
}

impl Default for ServerAddrs {
    fn default() -> Self {
        Self::new("127.0.0.1:14192".parse().expect("could not parse QUIC address/port"))
    }
}

// QUIC Configuration
#[derive(Clone)]
pub struct QuicConfig {
    pub datagram_receive_buffer_size: usize,
    pub datagram_send_buffer_size: usize,
    pub idle_timeout: Duration,
    pub keep_alive_interval: Duration,
    pub initial_rtt: Duration,
    pub certificate_config: CertificateConfig,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            datagram_receive_buffer_size: DATAGRAM_BUFFER_SIZE,
            datagram_send_buffer_size: DATAGRAM_BUFFER_SIZE,
            idle_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(5),
            initial_rtt: Duration::from_millis(100),
            certificate_config: CertificateConfig::SelfSigned {
                hostnames: vec!["localhost".to_string()],
            },
        }
    }
}

// Certificate Configuration
#[derive(Clone)]
pub enum CertificateConfig {
    /// Generate self-signed certificate (for development)
    SelfSigned { hostnames: Vec<String> },
    /// Provide certificate and private key as DER bytes
    FromBytes {
        cert_chain: Vec<Vec<u8>>,
        private_key: Vec<u8>,
    },
}

// Helper function to generate self-signed certificate
fn generate_self_signed_cert(
    hostnames: &[String],
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), String> {
    let cert = rcgen::generate_simple_self_signed(hostnames.to_vec())
        .map_err(|e| format!("Failed to generate certificate: {}", e))?;

    let key_der = cert.key_pair.serialize_der();
    let cert_der = cert
        .cert
        .der()
        .to_vec();

    Ok((
        vec![CertificateDer::from(cert_der)],
        PrivateKeyDer::try_from(key_der)
            .map_err(|_| "Failed to convert private key".to_string())?,
    ))
}
