//! Transport Tcp Option

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpClientOption {
    pub addr: String,
    pub port: u16,
    #[serde(default)]
    pub tcp_nodelay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpServerOption {
    pub listen: SocketAddr,
    #[serde(default)]
    pub tcp_nodelay: bool,
}
