//! WebSocket Client

use std::{
    net::{IpAddr, SocketAddr},
    pin::Pin,
    str::FromStr,
    sync::Arc,
    task::Poll,
};

use bytes::{Buf, Bytes};
use futures_util::{
    ready,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use http::Uri;
use rustls::ClientConfig as TlsClientConfig;
use tokio::{
    io::{AsyncBufRead, AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_tungstenite::{
    client_async_tls_with_config, tungstenite::Message, Connector as WsConnector, MaybeTlsStream,
    WebSocketStream,
};

use crate::{
    ClientError, ClientResult, ResolveError, Resolver, TlsClientOption, TransportClientTrait,
};

use super::WebSocketClientOption;

pub struct WebSocketClient {
    uri: Uri,
    addrs: Vec<SocketAddr>,
    ws_conn: WsConnector,
    tcp_nodelay: bool,
}

impl WebSocketClient {
    pub fn init(
        opt: WebSocketClientOption,
        tls_opt: Option<TlsClientOption>,
        resolver: &Resolver,
    ) -> ClientResult<Self> {
        let (ws_conn, uri) = if let Some(tls_opt) = tls_opt {
            let config: TlsClientConfig = tls_opt.try_into()?;
            let conn = WsConnector::Rustls(Arc::new(config));

            let uri = Uri::builder()
                .scheme("wss")
                .path_and_query(opt.path)
                .authority(format!("{}:{}", opt.addr, opt.port))
                .build()
                .map_err(|e| ClientError::Option(e.to_string()))?;

            (conn, uri)
        } else {
            (
                WsConnector::Plain,
                Uri::builder()
                    .scheme("ws")
                    .path_and_query(opt.path)
                    .authority(format!("{}:{}", opt.addr, opt.port))
                    .build()
                    .map_err(|e| ClientError::Option(e.to_string()))?,
            )
        };

        let addrs = match IpAddr::from_str(&opt.addr) {
            Ok(ip) => vec![(ip, opt.port).into()],
            Err(_) => resolver.block_resolve(&opt.addr, opt.port)?.collect(),
        };

        Ok(Self {
            addrs,
            uri,
            ws_conn,
            tcp_nodelay: opt.tcp_nodelay,
        })
    }
}

impl TransportClientTrait for WebSocketClient {
    type Stream = WebSocketClientStream;

    async fn connect(&self) -> ClientResult<Self::Stream> {
        let mut err = None;
        for addr in self.addrs.iter() {
            match tokio::net::TcpStream::connect(addr).await {
                Ok(stream) => {
                    if self.tcp_nodelay {
                        let _ = stream.set_nodelay(true);
                    }
                    let (socket, _) = client_async_tls_with_config(
                        &self.uri,
                        stream,
                        None,
                        Some(self.ws_conn.clone()),
                    )
                    .await
                    .map_err(|e| ClientError::Connect(e.to_string()))?;
                    let stream = WebSocketClientStream::new(socket);
                    return Ok(stream);
                }
                Err(e) => err = Some(e),
            }
        }

        if let Some(e) = err {
            Err(e.into())
        } else {
            Err(ResolveError::EmptyResolved.into())
        }
    }
}

pub struct WebSocketClientStream {
    tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    chunk: Option<Bytes>,
}

impl WebSocketClientStream {
    pub fn new(inner: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        let (tx, rx) = inner.split();
        Self {
            tx,
            rx,
            chunk: None,
        }
    }

    fn has_chunk(&self) -> bool {
        if let Some(ref chunk) = self.chunk {
            chunk.remaining() > 0
        } else {
            false
        }
    }
}

impl AsyncBufRead for WebSocketClientStream {
    fn poll_fill_buf(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<&[u8]>> {
        let this = self.get_mut();
        loop {
            if this.has_chunk() {
                let chunk = this.chunk.as_ref().unwrap();
                let buf = chunk.chunk();
                return Poll::Ready(Ok(buf));
            } else {
                let chunk = match this.rx.poll_next_unpin(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(None) => return Poll::Ready(Ok(&[])),
                    Poll::Ready(Some(Err(err))) => {
                        return Poll::Ready(Err(std::io::Error::other(err)))
                    }
                    Poll::Ready(Some(Ok(msg))) => match msg {
                        Message::Binary(data) => Bytes::from(data),
                        Message::Text(data) => Bytes::from(data),
                        _ => continue,
                    },
                };

                this.chunk = Some(chunk);
            }
        }
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        if amt > 0 {
            if let Some(chunk) = self.get_mut().chunk.as_mut() {
                chunk.advance(amt);
            }
        }
    }
}

impl AsyncRead for WebSocketClientStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        let inner_buf = match self.as_mut().poll_fill_buf(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
            Poll::Ready(Ok(buf)) => buf,
        };

        let len = std::cmp::min(inner_buf.len(), buf.remaining());
        buf.put_slice(&inner_buf[..len]);

        self.consume(len);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for WebSocketClientStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let this = self.get_mut();

        ready!(this
            .tx
            .poll_ready_unpin(cx)
            .map_err(|e| std::io::Error::other(e)))?;

        match this.tx.start_send_unpin(Message::binary(buf)) {
            Ok(()) => Poll::Ready(Ok(buf.len())),
            Err(e) => Poll::Ready(Err(std::io::Error::other(e))),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        self.get_mut()
            .tx
            .poll_flush_unpin(cx)
            .map_err(|e| std::io::Error::other(e))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        self.get_mut()
            .tx
            .poll_close_unpin(cx)
            .map_err(|e| std::io::Error::other(e))
    }
}
