use std::{
    sync::{Arc, Mutex},
};

use log::warn;
use naia_shared::IdentityToken;
use quinn::{Connection, RecvStream};
use tokio::sync::oneshot;

use crate::transport::{IdentityReceiver, IdentityReceiverResult};

// AuthIo
pub(crate) struct AuthIo {
    connection: Arc<Mutex<Option<Connection>>>,
    pending_auth: Option<PendingAuth>,
}

impl AuthIo {
    pub(crate) fn new(connection: Arc<Mutex<Option<Connection>>>) -> Self {
        Self {
            connection,
            pending_auth: None,
        }
    }

    pub(crate) fn initiate_auth(
        &mut self,
        auth_bytes_opt: Option<Vec<u8>>,
        auth_headers_opt: Option<Vec<(String, String)>>,
    ) {
        let conn_guard = self.connection.lock().unwrap();
        if let Some(connection) = conn_guard.as_ref() {
            self.pending_auth = Some(PendingAuth::new(
                connection.clone(),
                auth_bytes_opt,
                auth_headers_opt,
            ));
        } else {
            warn!("Cannot initiate auth: no connection established");
        }
    }

    fn receive(&mut self) -> IdentityReceiverResult {
        let Some(pending_auth) = self.pending_auth.as_mut() else {
            return IdentityReceiverResult::Waiting;
        };

        pending_auth.poll_response()
    }
}

// AuthReceiver
#[derive(Clone)]
pub(crate) struct AuthReceiver {
    auth_io: Arc<Mutex<AuthIo>>,
}

impl AuthReceiver {
    pub fn new(auth_io: Arc<Mutex<AuthIo>>) -> Self {
        Self { auth_io }
    }
}

impl IdentityReceiver for AuthReceiver {
    fn receive(&mut self) -> IdentityReceiverResult {
        let mut guard = self.auth_io.lock().unwrap();
        guard.receive()
    }
}

struct PendingAuth {
    receiver: oneshot::Receiver<Result<IdentityToken, AuthError>>,
}

impl PendingAuth {
    fn new(
        connection: Connection,
        auth_bytes_opt: Option<Vec<u8>>,
        _auth_headers_opt: Option<Vec<(String, String)>>,
    ) -> Self {
        let (tx, rx) = oneshot::channel::<Result<IdentityToken, AuthError>>();

        tokio::spawn(async move {
            let result = send_auth_and_receive_token(connection, auth_bytes_opt).await;
            let _ = tx.send(result);
        });

        Self { receiver: rx }
    }

    pub fn poll_response(&mut self) -> IdentityReceiverResult {
        match self.receiver.try_recv() {
            Ok(Ok(identity_token)) => IdentityReceiverResult::Success(identity_token),
            Ok(Err(AuthError::Rejected)) => IdentityReceiverResult::ErrorResponseCode(401),
            Ok(Err(AuthError::ConnectionClosed)) => IdentityReceiverResult::ErrorResponseCode(500),
            Ok(Err(AuthError::StreamError(_))) => IdentityReceiverResult::ErrorResponseCode(500),
            Err(oneshot::error::TryRecvError::Empty) => IdentityReceiverResult::Waiting,
            Err(oneshot::error::TryRecvError::Closed) => {
                IdentityReceiverResult::ErrorResponseCode(500)
            }
        }
    }
}

async fn send_auth_and_receive_token(
    connection: Connection,
    auth_bytes_opt: Option<Vec<u8>>,
) -> Result<IdentityToken, AuthError> {
    // Open bidirectional stream for auth
    let (mut send, recv) = connection
        .open_bi()
        .await
        .map_err(|e| AuthError::StreamError(format!("Failed to open auth stream: {}", e)))?;

    // Send auth data
    if let Some(auth_bytes) = auth_bytes_opt {
        send.write_all(&auth_bytes)
            .await
            .map_err(|e| AuthError::StreamError(format!("Failed to send auth data: {}", e)))?;
    }
    send.finish().map_err(|e| AuthError::StreamError(format!("Failed to finish auth stream: {}", e)))?;

    // Close the send side, keep receive open for response
    drop(send);

    // Wait for server's auth response on a unidirectional stream
    let identity_token = receive_identity_token(connection).await?;

    Ok(identity_token)
}

async fn receive_identity_token(connection: Connection) -> Result<IdentityToken, AuthError> {
    // Accept unidirectional stream from server with identity token
    let mut recv: RecvStream = connection
        .accept_uni()
        .await
        .map_err(|e| {
            // Check if connection was closed (likely rejected)
            if matches!(e, quinn::ConnectionError::ApplicationClosed(_)) {
                AuthError::Rejected
            } else {
                AuthError::StreamError(format!("Failed to accept identity stream: {}", e))
            }
        })?;

    // Read identity token (max 1KB)
    let token_bytes = recv
        .read_to_end(1024)
        .await
        .map_err(|e| AuthError::StreamError(format!("Failed to read identity token: {}", e)))?;

    let token_str = String::from_utf8(token_bytes)
        .map_err(|e| AuthError::StreamError(format!("Invalid UTF-8 in token: {}", e)))?;

    Ok(token_str)
}

#[derive(Debug)]
enum AuthError {
    Rejected,
    ConnectionClosed,
    StreamError(String),
}
