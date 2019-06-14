// TODO: use the nightly opportunity to clear up some deprecations that may be coming our way

use crate::sans_io::Client as SansIoClient;
use crate::{Command, RBytes, RedisError, StructuredCommand};
use futures::io::{AsyncBufReadExt, BufReader, Result as IoResult};
use runtime::net::TcpStream;
use std::net::ToSocketAddrs;
use std::thread;

pub struct Client {
    parser: SansIoClient,
    writer: TcpStream,
}

impl Client {
    pub async fn new(address: impl ToSocketAddrs) -> IoResult<Self> {
        let writer = TcpStream::connect(address).await?;
        let (parser, _) = SansIoClient::new();

        Ok(Self { parser, writer })
    }

    pub async fn with_auth<'a>(
        address: impl ToSocketAddrs,
        pass: impl Into<RBytes<'a>>,
    ) -> IoResult<Self> {
        Self::new(address).await
    }

    pub fn issue<Cmd>(&mut self, cmd: Cmd) -> Result<<Cmd as StructuredCommand>::Output, RedisError>
    where
        Cmd: StructuredCommand,
    {
        unimplemented!("cannot yet use the async client")
    }
}
