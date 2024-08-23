//! WebSocket Transport Option

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketServerOption {
    pub listen: SocketAddr,
    pub path: String,
    #[serde(default)]
    pub tcp_nodelay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketClientOption {
    pub addr: String,
    pub port: u16,
    pub path: String,
    #[serde(default)]
    pub tcp_nodelay: bool,
}
