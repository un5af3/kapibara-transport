//! Transport client

use crate::{
    empty::{EmptyClient, EmptyStream},
    option::ClientOption,
    stream_traits_enum,
    tcp::{TcpClient, TcpStream},
    websocket::{WebSocketClient, WebSocketClientStream},
    ClientResult, Resolver, TransportClientOption, TransportClientTrait,
};

macro_rules! transport_client_enum {
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

        impl TransportClientTrait for $name
        {
            type Stream = TransportClientStream;

            async fn connect(&self) -> ClientResult<Self::Stream> {
                match self {
                    $(
                        $name::$id(cli) => Ok(cli.connect().await?.into()),
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
    pub enum TransportClientStream {
        Empty(EmptyStream),
        Tcp(TcpStream),
        Ws(WebSocketClientStream),
    }
}

impl TransportClientStream {
    pub fn is_emtpy(&self) -> bool {
        matches!(self, Self::Empty(_))
    }
}

transport_client_enum! {
    pub enum TransportClient {
        Empty(EmptyClient),
        Tcp(TcpClient),
        Ws(WebSocketClient),
    }
}

impl TransportClient {
    pub fn init(trans_opt: TransportClientOption, resolver: &Resolver) -> ClientResult<Self> {
        match trans_opt.opt {
            ClientOption::Empty => Ok(EmptyClient.into()),
            ClientOption::Tcp(opt) => Ok(TcpClient::init(opt, trans_opt.tls, resolver)?.into()),
            ClientOption::Ws(opt) => {
                Ok(WebSocketClient::init(opt, trans_opt.tls, resolver)?.into())
            }
        }
    }
}
