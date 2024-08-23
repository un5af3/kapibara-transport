//! Transport Tcp Stream

use tokio::net::TcpStream as TokioTcpStream;
use tokio_rustls::TlsStream;

use crate::stream_traits_enum;

stream_traits_enum! {
    pub enum TcpStream {
        Raw(TokioTcpStream),
        Tls(TlsStream<TokioTcpStream>),
    }
}
