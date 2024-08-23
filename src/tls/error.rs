//! Tls Error Handle

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TlsError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid certificate: {0}")]
    InvalidCert(String),
    #[error("invalid private key: {0}")]
    InvalidKey(String),
}
