//! WebSocket Transport Server

use std::{net::SocketAddr, pin::Pin, sync::Arc, task::Poll};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, State, WebSocketUpgrade,
    },
    routing::get,
    Router,
};
use axum_server::{
    accept::NoDelayAcceptor,
    tls_rustls::{RustlsAcceptor, RustlsConfig},
};
use bytes::{Buf, Bytes};
use futures_util::{
    ready,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite};

use crate::{ServerResult, TlsServerOption, TransportServerCallback, TransportServerTrait};

use super::WebSocketServerOption;

pub struct WebSocketServer {
    path: String,
    listen: SocketAddr,
    tls_cfg: Option<RustlsConfig>,
    tcp_nodelay: bool,
}

impl WebSocketServer {
    pub fn init(
        opt: WebSocketServerOption,
        tls_opt: Option<TlsServerOption>,
    ) -> ServerResult<Self> {
        let tls_cfg = if let Some(tls_opt) = tls_opt {
            Some(RustlsConfig::from_config(Arc::new(tls_opt.try_into()?)))
        } else {
            None
        };

        Ok(Self {
            path: opt.path,
            listen: opt.listen,
            tls_cfg,
            tcp_nodelay: opt.tcp_nodelay,
        })
    }
}

impl TransportServerTrait for WebSocketServer {
    fn local_addr(&self) -> Option<SocketAddr> {
        Some(self.listen)
    }

    async fn serve<C: TransportServerCallback>(&self, callback: C) -> ServerResult<()> {
        let svc = Router::new()
            .route(
                &self.path,
                get(
                    |ws: WebSocketUpgrade,
                     ConnectInfo(addr): ConnectInfo<SocketAddr>,
                     State(c): State<C>| async move {
                        ws.on_upgrade(move |socket| async move {
                            let stream = WebSocketServerStream::new(socket);
                            let _ = c.handle(stream, Some(addr)).await;
                        })
                    },
                ),
            )
            .with_state(callback);

        if let Some(ref tls_cfg) = self.tls_cfg {
            if self.tcp_nodelay {
                let acceptor =
                    RustlsAcceptor::new(tls_cfg.clone()).acceptor(NoDelayAcceptor::new());
                axum_server::bind(self.listen)
                    .acceptor(acceptor)
                    .serve(svc.into_make_service_with_connect_info::<SocketAddr>())
                    .await?;
            } else {
                axum_server::bind_rustls(self.listen, tls_cfg.clone())
                    .serve(svc.into_make_service_with_connect_info::<SocketAddr>())
                    .await?
            }
        } else {
            if self.tcp_nodelay {
                axum_server::bind(self.listen)
                    .acceptor(NoDelayAcceptor::new())
                    .serve(svc.into_make_service_with_connect_info::<SocketAddr>())
                    .await?
            } else {
                axum_server::bind(self.listen)
                    .serve(svc.into_make_service_with_connect_info::<SocketAddr>())
                    .await?
            }
        }

        Ok(())
    }
}

pub struct WebSocketServerStream {
    tx: SplitSink<WebSocket, Message>,
    rx: SplitStream<WebSocket>,
    chunk: Option<Bytes>,
}

impl WebSocketServerStream {
    pub fn new(socket: WebSocket) -> Self {
        let (tx, rx) = socket.split();

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

impl AsyncBufRead for WebSocketServerStream {
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

impl AsyncRead for WebSocketServerStream {
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

impl AsyncWrite for WebSocketServerStream {
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

        match this.tx.start_send_unpin(Message::Binary(buf.into())) {
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
