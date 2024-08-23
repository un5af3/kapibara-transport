//! Kapibara Error Handle

use thiserror::Error;

use crate::{ResolveError, TlsError};

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("io error ({0})")]
    Io(#[from] std::io::Error),
    #[error("resolve error ({0})")]
    Dns(#[from] ResolveError),
    #[error("tls error ({0})")]
    Tls(#[from] TlsError),
    #[error("option error ({0})")]
    Option(String),
    #[error("connect error ({0})")]
    Connect(String),
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("io error ({0})")]
    Io(#[from] std::io::Error),
    #[error("tls error ({0})")]
    Tls(#[from] TlsError),
    #[error("option error ({0})")]
    Option(String),
    #[error("serve error ({0})")]
    Serve(String),
}

impl ServerError {
    pub fn is_closed(&self) -> bool {
        if let ServerError::Io(err) = self {
            match err.kind() {
                std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::NotConnected
                | std::io::ErrorKind::BrokenPipe
                | std::io::ErrorKind::ConnectionReset => true,
                _ => false,
            }
        } else {
            false
        }
    }
}
