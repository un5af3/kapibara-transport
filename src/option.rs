//! Transport Option

use serde::{Deserialize, Serialize};

use crate::{
    tcp::{TcpClientOption, TcpServerOption},
    websocket::{WebSocketClientOption, WebSocketServerOption},
    TlsClientOption, TlsServerOption,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransportClientOption {
    #[serde(default)]
    pub opt: ClientOption,
    #[serde(default)]
    pub tls: Option<TlsClientOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransportServerOption {
    pub opt: ServerOption,
    #[serde(default)]
    pub tls: Option<TlsServerOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientOption {
    Empty,
    Tcp(TcpClientOption),
    Ws(WebSocketClientOption),
}

impl Default for ClientOption {
    fn default() -> Self {
        Self::Empty
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerOption {
    Tcp(TcpServerOption),
    Ws(WebSocketServerOption),
}

/*
impl ServerOption {
    pub fn name(&self) -> &str {}

    pub fn addr(&self) -> SocketAddr {}
}
*/
