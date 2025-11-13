mod auth;
mod data;

pub use data::Socket;
pub use crate::transport::quic::data::{QuicConfig, CertificateVerification};
