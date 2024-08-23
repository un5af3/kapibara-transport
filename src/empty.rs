//! Empty Client

use crate::{ClientResult, TransportClientTrait};

pub struct EmptyClient;

pub type EmptyStream = tokio::io::Empty;

impl TransportClientTrait for EmptyClient {
    type Stream = EmptyStream;

    async fn connect(&self) -> ClientResult<Self::Stream> {
        Ok(tokio::io::empty())
    }
}
