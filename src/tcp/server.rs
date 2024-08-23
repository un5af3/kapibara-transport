//! Transport Tcp Server

use std::{net::SocketAddr, sync::Arc};

use rustls::ServerConfig as TlsServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::{TlsAcceptor, TlsStream};

use crate::{
    ServerError, ServerResult, TlsServerOption, TransportServerCallback, TransportServerTrait,
};

use super::{TcpServerOption, TcpStream};

pub struct TcpServer {
    local_addr: SocketAddr,
    tls_acceptor: Option<TlsAcceptor>,
    tcp_nodelay: bool,
}

impl TcpServer {
    pub fn init(opt: TcpServerOption, tls_opt: Option<TlsServerOption>) -> ServerResult<Self> {
        let tls_acceptor = if let Some(tls_opt) = tls_opt {
            let config: TlsServerConfig = tls_opt.try_into()?;
            Some(TlsAcceptor::from(Arc::new(config)))
        } else {
            None
        };

        Ok(Self {
            local_addr: opt.listen,
            tls_acceptor,
            tcp_nodelay: opt.tcp_nodelay,
        })
    }
}

impl TransportServerTrait for TcpServer {
    fn local_addr(&self) -> Option<SocketAddr> {
        Some(self.local_addr)
    }

    async fn serve<C: TransportServerCallback>(&self, callback: C) -> ServerResult<()> {
        let listener = TcpListener::bind(self.local_addr).await?;

        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok((s, a)) => {
                    if self.tcp_nodelay {
                        let _ = s.set_nodelay(true);
                    }
                    let s = if let Some(ref acceptor) = self.tls_acceptor {
                        match acceptor.accept(s).await {
                            Ok(s) => TcpStream::Tls(TlsStream::Server(s)),
                            Err(e) => {
                                log::warn!("tls handshake failed {}", e);
                                continue;
                            }
                        }
                    } else {
                        TcpStream::Raw(s)
                    };

                    (s, a)
                }
                Err(err) => {
                    let err: ServerError = err.into();
                    if err.is_closed() {
                        return Err(err);
                    }

                    log::error!("tcp server error: {}", err);
                    continue;
                }
            };

            let callback_clone = callback.clone();
            let stream: TcpStream = stream.into();
            tokio::spawn(async move { callback_clone.handle(stream, Some(peer_addr)).await });
        }
    }
}
