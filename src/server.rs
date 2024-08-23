//! Transport Server
use std::net::SocketAddr;

use crate::{
    option::ServerOption,
    stream_traits_enum,
    tcp::{TcpServer, TcpStream},
    websocket::{WebSocketServer, WebSocketServerStream},
    ServerResult, TransportServerCallback, TransportServerOption, TransportServerTrait,
};

macro_rules! transport_server_enum {
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
        $v enum $name {
            $(
                $(#[$item_meta])*
                $id($id_ty),
            )+
        }

        impl $name {
            pub fn name(&self) -> &str {
                match self {
                    $(
                        $name::$id(_) => stringify!($id),
                    )+
                }
            }
        }

        impl TransportServerTrait for $name
        {
            fn local_addr(&self) -> Option<SocketAddr> {
                match self {
                    $(
                        $name::$id(svc) => svc.local_addr(),
                    )+
                }
            }

            async fn serve<C: TransportServerCallback>(&self, callback: C) -> ServerResult<()> {
                match self {
                    $(
                        $name::$id(svc) => svc.serve(callback).await,
                    )+
                }
            }
        }

        $(
            impl From<$id_ty> for $name {
                fn from(s: $id_ty) -> $name {
                    $name::$id(s)
                }
            }
        )+
    };
}

stream_traits_enum! {
    pub enum TransportServerStream {
        Tcp(TcpStream),
        Ws(WebSocketServerStream),
    }
}

transport_server_enum! {
    pub enum TransportServer {
        Tcp(TcpServer),
        Ws(WebSocketServer),
    }
}

impl TransportServer {
    pub fn init(trans_opt: TransportServerOption) -> ServerResult<Self> {
        match trans_opt.opt {
            ServerOption::Tcp(opt) => Ok(TcpServer::init(opt, trans_opt.tls)?.into()),
            ServerOption::Ws(opt) => Ok(WebSocketServer::init(opt, trans_opt.tls)?.into()),
        }
    }
}
