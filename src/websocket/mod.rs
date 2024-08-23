//! WebSocket Transport

pub mod option;
pub use option::{WebSocketClientOption, WebSocketServerOption};

pub mod server;
pub use server::{WebSocketServer, WebSocketServerStream};

pub mod client;
pub use client::{WebSocketClient, WebSocketClientStream};

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use crate::{
        Resolver, TlsCertOption, TlsClientOption, TlsServerOption, TransportClientTrait,
        TransportServerCallback, TransportServerTrait,
    };

    use super::*;

    #[derive(Debug, Clone)]
    struct MockServerCallback;

    impl TransportServerCallback for MockServerCallback {
        async fn handle<S>(&self, mut stream: S, addr: Option<std::net::SocketAddr>)
        where
            S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync,
        {
            if let Some(addr) = addr {
                println!("ws connection from {}", addr);
            }

            let mut buf = [0u8; 1024];
            for _ in 0..100 {
                let _ = stream.write(&b"f".repeat(1024 * 100)).await.unwrap();
                let _ = stream.flush().await;
                for _ in 0..100 {
                    let n = stream.read(&mut buf).await.unwrap();
                    assert_eq!(&buf[..n], b"k".repeat(1024));
                }
            }
        }
    }

    #[tokio::test]
    async fn test_ws_client() {
        tokio::spawn(async move {
            let opt = WebSocketServerOption {
                listen: "127.0.0.1:9876".parse().unwrap(),
                path: "/test".into(),
                tcp_nodelay: true,
            };

            let tls_opt = TlsServerOption {
                alpn: vec![],
                certificate: TlsCertOption::File {
                    cert: "certs/test.crt".into(),
                    key: "certs/test.key".into(),
                },
            };

            let srv = WebSocketServer::init(opt, Some(tls_opt)).unwrap();

            if let Err(err) = srv.serve(MockServerCallback).await {
                panic!("{}", err);
            }
        });

        let opt = WebSocketClientOption {
            addr: "127.0.0.1".into(),
            port: 9876,
            path: "/test".into(),
            tcp_nodelay: false,
        };

        let tls_opt = TlsClientOption {
            insecure: true,
            alpn: vec![],
            enable_sni: false,
            server_name: String::new(),
        };

        let resolver = Resolver::default();
        let cli = WebSocketClient::init(opt, Some(tls_opt), &resolver).unwrap();
        let mut ws_stream = cli.connect().await.unwrap();
        let mut buf = [0u8; 1024];
        for _ in 0..100 {
            for _ in 0..100 {
                let n = ws_stream.read(&mut buf).await.unwrap();
                assert_eq!(&buf[..n], b"f".repeat(1024));
            }
            let _ = ws_stream.write(&b"k".repeat(1024 * 100)).await.unwrap();
            let _ = ws_stream.flush().await;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
