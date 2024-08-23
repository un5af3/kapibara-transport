//! Tcp Transport client

use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use rustls::{pki_types::ServerName, ClientConfig as TlsClientConfig};
use tokio::net::TcpStream as TokioTcpStream;
use tokio_rustls::{TlsConnector, TlsStream};

use crate::{
    ClientError, ClientResult, ResolveError, Resolver, TlsClientOption, TransportClientTrait,
};

use super::{TcpClientOption, TcpStream};

pub struct TcpClient {
    addr: Vec<SocketAddr>,
    tls_conn: Option<(TlsConnector, ServerName<'static>)>,
    tcp_nodelay: bool,
}

impl TcpClient {
    pub fn init(
        opt: TcpClientOption,
        tls_opt: Option<TlsClientOption>,
        resolver: &Resolver,
    ) -> ClientResult<Self> {
        let tls_conn = if let Some(tls_opt) = tls_opt {
            let server_name = ServerName::try_from(if tls_opt.server_name.is_empty() {
                opt.addr.clone()
            } else {
                tls_opt.server_name.clone()
            })
            .map_err(|e| ClientError::Option(e.to_string()))?;

            let config: TlsClientConfig = tls_opt.try_into()?;
            let conn = TlsConnector::from(Arc::new(config));
            Some((conn, server_name))
        } else {
            None
        };

        let addr = match IpAddr::from_str(&opt.addr) {
            Ok(ip) => vec![(ip, opt.port).into()],
            Err(_) => {
                let res = resolver.block_resolve(opt.addr, opt.port)?;
                res.collect()
            }
        };

        if addr.is_empty() {
            return Err(ClientError::Option("unknown address".to_owned()));
        }

        Ok(Self {
            addr,
            tls_conn,
            tcp_nodelay: opt.tcp_nodelay,
        })
    }
}

impl TransportClientTrait for TcpClient {
    type Stream = TcpStream;

    async fn connect(&self) -> ClientResult<Self::Stream> {
        let mut err = None;
        for addr in self.addr.iter() {
            match TokioTcpStream::connect(addr).await {
                Ok(s) => {
                    if self.tcp_nodelay {
                        let _ = s.set_nodelay(true);
                    }
                    let stream = if let Some((ref tls_conn, ref server_name)) = self.tls_conn {
                        let stream = tls_conn.connect(server_name.clone(), s).await?;
                        TcpStream::Tls(TlsStream::Client(stream))
                    } else {
                        TcpStream::Raw(s)
                    };

                    return Ok(stream);
                }
                Err(e) => err = Some(e),
            }
        }

        if let Some(err) = err {
            Err(err.into())
        } else {
            Err(ResolveError::EmptyResolved.into())
        }
    }
}
