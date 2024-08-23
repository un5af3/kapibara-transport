//! Kapibara Transport Library
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};

pub mod error;
pub use error::{ClientError, ServerError};

pub mod option;
pub use option::{TransportClientOption, TransportServerOption};

pub mod client;
pub use client::{TransportClient, TransportClientStream};

pub mod server;
pub use server::{TransportServer, TransportServerStream};

pub mod tls;
pub use tls::{TlsCertOption, TlsClientOption, TlsError, TlsServerOption};

pub mod dns;
pub use dns::{ResolveError, ResolveOption, Resolver};

pub mod empty;
pub mod tcp;
pub mod websocket;

pub type ClientResult<T> = std::result::Result<T, ClientError>;
pub type ServerResult<T> = std::result::Result<T, ServerError>;

#[trait_variant::make(TransportServerTrait: Send + Sync)]
pub trait LocalTransportServerTrait {
    fn local_addr(&self) -> Option<SocketAddr>;
    async fn serve<C>(&self, callback: C) -> ServerResult<()>
    where
        C: TransportServerCallback;
}

#[trait_variant::make(TransportServerCallback: Send + Sync)]
pub trait LocalTransportServerCallback: 'static + Clone {
    async fn handle<S>(&self, stream: S, addr: Option<SocketAddr>)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + Sync;
}

#[trait_variant::make(TransportClientTrait: Send + Sync)]
pub trait LocalTransportClientTrait {
    type Stream: AsyncRead + AsyncWrite + Unpin + Send + Sync;

    async fn connect(&self) -> ClientResult<Self::Stream>;
}

#[macro_export]
macro_rules! stream_traits_enum {
    {
        $(#[$meta:meta])*
        $v:vis enum $name:ident
        {
            $(
                $(#[$item_meta:meta])*
                $id:ident($id_ty:ty),
            )+
        }
    } => {
        $(#[$meta])*
        $v enum $name
        {
            $(
                $(#[$item_meta])*
                $id($id_ty),
            )+
        }

        impl tokio::io::AsyncRead for $name
        {
            #[inline]
            fn poll_read(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
                buf: &mut tokio::io::ReadBuf<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                match self.get_mut() {
                    $(
                        $name::$id(val) => std::pin::Pin::new(val).poll_read(cx, buf),
                    )+
                }
            }
        }

        impl tokio::io::AsyncWrite for $name
        {
            #[inline]
            fn poll_write(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
                buf: &[u8],
            ) -> std::task::Poll<std::io::Result<usize>> {
                match self.get_mut() {
                    $(
                        $name::$id(val) => std::pin::Pin::new(val).poll_write(cx, buf),
                    )+
                }
            }

            #[inline]
            fn poll_flush(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                match self.get_mut() {
                    $(
                        $name::$id(val) => std::pin::Pin::new(val).poll_flush(cx),
                    )+
                }
            }

            #[inline]
            fn poll_shutdown(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                match self.get_mut() {
                    $(
                        $name::$id(val) => std::pin::Pin::new(val).poll_shutdown(cx),
                    )+
                }
            }
        }

        $(
            impl From<$id_ty> for $name {
                fn from(val: $id_ty) -> $name {
                    $name::$id(val)
                }
            }
        )+
    };
}
