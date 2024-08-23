//! Dns Error

use hickory_resolver::error::ResolveError as HickoryResolveError;
use thiserror::Error;
use tokio::time::error::Elapsed;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("empty resolved")]
    EmptyResolved,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("resolve error: {0}")]
    Resolve(#[from] HickoryResolveError),
    #[error("resolve timeout")]
    Timeout(#[from] Elapsed),
    #[error("init error: {0}")]
    Initialize(String),
}
