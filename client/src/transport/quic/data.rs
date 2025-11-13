use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use quinn::{ClientConfig, Connection, Endpoint, TransportConfig, VarInt};
use rustls::pki_types::{CertificateDer, ServerName};

use crate::transport::{
    quic::auth::{AuthIo, AuthReceiver},
    IdentityReceiver, PacketReceiver as TransportPacketReceiver, PacketSender as TransportSender, RecvError, SendError,
    ServerAddr as TransportAddr, Socket as TransportSocket,
};

// Constants
const DATAGRAM_BUFFER_SIZE: usize = 10_000_000; // 10MB

// Socket
pub struct Socket {
    server_addr: SocketAddr,
    server_name: String,
    connection: Arc<Mutex<Option<Connection>>>,
    config: QuicConfig,
}

impl Socket {
    pub fn new(server_url: &str, config: QuicConfig) -> Self {
        // Parse server URL to extract address and name
        let (server_addr, server_name) = parse_server_url(server_url);

        Self {
            server_addr,
            server_name,
            connection: Arc::new(Mutex::new(None)),
            config,
        }
    }

    fn connect_inner(
        self: Box<Self>,
        auth_bytes_opt: Option<Vec<u8>>,
        auth_headers_opt: Option<Vec<(String, String)>>,
    ) -> (
        Box<dyn IdentityReceiver>,
        Box<dyn TransportSender>,
        Box<dyn TransportPacketReceiver>,
    ) {
        // Create auth IO
        let auth_io = Arc::new(Mutex::new(AuthIo::new(self.connection.clone())));

        // Spawn connection task
        spawn_connection_task(
            self.server_addr,
            self.server_name.clone(),
            self.connection.clone(),
            self.config.clone(),
            auth_io.clone(),
            auth_bytes_opt,
            auth_headers_opt,
        );

        let id_receiver = AuthReceiver::new(auth_io);
        let packet_sender = Box::new(PacketSender::new(self.connection.clone()));
        let packet_receiver = Box::new(PacketReceiver::new(self.connection.clone()));

        (Box::new(id_receiver), packet_sender, packet_receiver)
    }
}

impl Into<Box<dyn TransportSocket>> for Socket {
    fn into(self) -> Box<dyn TransportSocket> {
        Box::new(self)
    }
}

impl TransportSocket for Socket {
    fn connect(
        self: Box<Self>,
    ) -> (
        Box<dyn IdentityReceiver>,
        Box<dyn TransportSender>,
        Box<dyn TransportPacketReceiver>,
    ) {
        self.connect_inner(None, None)
    }

    fn connect_with_auth(
        self: Box<Self>,
        auth_bytes: Vec<u8>,
    ) -> (
        Box<dyn IdentityReceiver>,
        Box<dyn TransportSender>,
        Box<dyn TransportPacketReceiver>,
    ) {
        self.connect_inner(Some(auth_bytes), None)
    }

    fn connect_with_auth_headers(
        self: Box<Self>,
        auth_headers: Vec<(String, String)>,
    ) -> (
        Box<dyn IdentityReceiver>,
        Box<dyn TransportSender>,
        Box<dyn TransportPacketReceiver>,
    ) {
        self.connect_inner(None, Some(auth_headers))
    }

    fn connect_with_auth_and_headers(
        self: Box<Self>,
        auth_bytes: Vec<u8>,
        auth_headers: Vec<(String, String)>,
    ) -> (
        Box<dyn IdentityReceiver>,
        Box<dyn TransportSender>,
        Box<dyn TransportPacketReceiver>,
    ) {
        self.connect_inner(Some(auth_bytes), Some(auth_headers))
    }
}

fn spawn_connection_task(
    server_addr: SocketAddr,
    server_name: String,
    connection_slot: Arc<Mutex<Option<Connection>>>,
    config: QuicConfig,
    auth_io: Arc<Mutex<AuthIo>>,
    auth_bytes_opt: Option<Vec<u8>>,
    auth_headers_opt: Option<Vec<(String, String)>>,
) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        runtime.block_on(async {
            match establish_connection(server_addr, &server_name, &config).await {
                Ok(connection) => {
                    log::info!("QUIC connection established to {}", server_addr);

                    // Store connection
                    {
                        let mut conn_guard = connection_slot.lock().unwrap();
                        *conn_guard = Some(connection.clone());
                    }

                    // Initiate auth
                    {
                        let mut auth_guard = auth_io.lock().unwrap();
                        auth_guard.initiate_auth(auth_bytes_opt, auth_headers_opt);
                    }

                    // Keep connection alive (don't drop it)
                    let _ = connection.closed().await;
                    log::info!("QUIC connection closed");
                }
                Err(e) => {
                    log::error!("Failed to establish QUIC connection: {}", e);
                }
            }
        });
    });
}

async fn establish_connection(
    server_addr: SocketAddr,
    server_name: &str,
    config: &QuicConfig,
) -> Result<Connection, String> {
    // Create client config
    let mut client_crypto = match &config.cert_verification {
        CertificateVerification::System => {
            rustls::ClientConfig::builder()
                .with_root_certificates(rustls::RootCertStore::empty())
                .with_no_client_auth()
        }
        CertificateVerification::SkipVerification => {
            let mut crypto = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
                .with_no_client_auth();
            crypto.alpn_protocols = vec![b"naia-quic".to_vec()];
            crypto
        }
    };

    client_crypto.alpn_protocols = vec![b"naia-quic".to_vec()];

    let mut client_config = ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)
            .map_err(|e| format!("Failed to create QUIC client config: {}", e))?,
    ));

    // Configure transport for low latency
    let mut transport_config = TransportConfig::default();
    transport_config.datagram_receive_buffer_size(Some(config.datagram_receive_buffer_size));
    transport_config.datagram_send_buffer_size(config.datagram_send_buffer_size);
    transport_config.max_idle_timeout(Some(
        VarInt::from_u64(config.idle_timeout.as_millis() as u64)
            .map_err(|_| "Invalid idle timeout".to_string())?
            .into(),
    ));
    transport_config.keep_alive_interval(Some(config.keep_alive_interval));
    transport_config.initial_rtt(config.initial_rtt);

    client_config.transport_config(Arc::new(transport_config));

    // Create endpoint
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())
        .map_err(|e| format!("Failed to create client endpoint: {}", e))?;
    endpoint.set_default_client_config(client_config);

    // Connect
    let connection = endpoint
        .connect(server_addr, server_name)
        .map_err(|e| format!("Failed to initiate connection: {}", e))?
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    Ok(connection)
}

// Packet Sender
struct PacketSender {
    connection: Arc<Mutex<Option<Connection>>>,
}

impl PacketSender {
    pub fn new(connection: Arc<Mutex<Option<Connection>>>) -> Self {
        Self { connection }
    }
}

impl TransportSender for PacketSender {
    fn send(&self, payload: &[u8]) -> Result<(), SendError> {
        let conn_guard = self.connection.lock().unwrap();
        if let Some(connection) = conn_guard.as_ref() {
            connection
                .send_datagram(payload.to_vec().into())
                .map_err(|_| SendError)?;
            Ok(())
        } else {
            Err(SendError)
        }
    }

    fn server_addr(&self) -> TransportAddr {
        let conn_guard = self.connection.lock().unwrap();
        if let Some(connection) = conn_guard.as_ref() {
            TransportAddr::Found(connection.remote_address())
        } else {
            TransportAddr::Finding
        }
    }
}

// Packet Receiver
#[derive(Clone)]
pub(crate) struct PacketReceiver {
    connection: Arc<Mutex<Option<Connection>>>,
    buffer: Vec<u8>,
}

impl PacketReceiver {
    pub fn new(connection: Arc<Mutex<Option<Connection>>>) -> Self {
        Self {
            connection,
            buffer: Vec::new(),
        }
    }
}

impl TransportPacketReceiver for PacketReceiver {
    fn receive(&mut self) -> Result<Option<&[u8]>, RecvError> {
        let conn_guard = self.connection.lock().unwrap();
        if let Some(connection) = conn_guard.as_ref() {
            // Try to receive datagram (non-blocking)
            match connection.read_datagram() {
                Ok(data) => {
                    self.buffer = data.to_vec();
                    Ok(Some(&self.buffer))
                }
                Err(quinn::ConnectionError::ApplicationClosed(_)) => Err(RecvError),
                Err(_) => Ok(None), // No datagram available
            }
        } else {
            Ok(None)
        }
    }

    fn server_addr(&self) -> TransportAddr {
        let conn_guard = self.connection.lock().unwrap();
        if let Some(connection) = conn_guard.as_ref() {
            TransportAddr::Found(connection.remote_address())
        } else {
            TransportAddr::Finding
        }
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
    pub cert_verification: CertificateVerification,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            datagram_receive_buffer_size: DATAGRAM_BUFFER_SIZE,
            datagram_send_buffer_size: DATAGRAM_BUFFER_SIZE,
            idle_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(5),
            initial_rtt: Duration::from_millis(100),
            cert_verification: CertificateVerification::SkipVerification, // Dev-friendly default
        }
    }
}

// Certificate Verification
#[derive(Clone)]
pub enum CertificateVerification {
    /// Use system certificate store
    System,
    /// Skip certificate verification (dev only!)
    SkipVerification,
}

// Helper to skip certificate verification (for development with self-signed certs)
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

// Helper to parse server URL
fn parse_server_url(url: &str) -> (SocketAddr, String) {
    // Simple parsing: expect format like "localhost:14192" or "127.0.0.1:14192"
    let addr: SocketAddr = url
        .parse()
        .unwrap_or_else(|_| {
            // Try with scheme
            if url.starts_with("quic://") {
                url[7..].parse()
            } else {
                url.parse()
            }
        })
        .expect("Invalid server URL");

    // Extract hostname for SNI
    let hostname = if let Ok(host_port) = url.parse::<SocketAddr>() {
        host_port.ip().to_string()
    } else if url.contains("://") {
        url.split("://").nth(1).unwrap_or(url).split(':').next().unwrap_or("localhost").to_string()
    } else {
        url.split(':').next().unwrap_or("localhost").to_string()
    };

    (addr, hostname)
}
