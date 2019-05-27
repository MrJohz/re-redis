use crate::sans_io::SansIoClient;
use crate::types::redis_values::ConversionError;
use crate::{RedisError, RedisResult, StructuredCommand};
use std::convert::TryFrom;
use std::io::{BufRead, BufReader, BufWriter, Result as IoResult, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;

pub struct Client {
    writer: BufWriter<TcpStream>,
    parser: SansIoClient,
}

impl Client {
    pub fn new(address: impl ToSocketAddrs) -> IoResult<Self> {
        let stream = TcpStream::connect(address)?;
        let writer = BufWriter::new(stream.try_clone()?);
        let (parser, tx_bytes) = SansIoClient::new();

        thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            let mut buffer = String::new();

            loop {
                match reader.read_line(&mut buffer) {
                    Ok(_) => {
                        tx_bytes.send(Ok(buffer.clone().into())).unwrap();
                        buffer.clear();
                    }
                    Err(err) => {
                        tx_bytes.send(Err(err)).unwrap();
                        break;
                    }
                }
            }

            // TODO: figure out why this closes when the tests are finished
        });

        Ok(Self { parser, writer })
    }

    pub fn issue_command<Cmd>(
        &mut self,
        cmd: Cmd,
    ) -> Result<<Cmd as StructuredCommand>::Output, RedisError>
    where
        Cmd: StructuredCommand,
        Cmd::Output: TryFrom<RedisResult, Error = ConversionError>,
    {
        let bytes = self.parser.issue_command(cmd);
        self.writer
            .write(&bytes)
            .map_err(RedisError::ConnectionError)?;
        self.writer.flush().map_err(RedisError::ConnectionError)?;
        self.parser
            .get_response::<<Cmd as StructuredCommand>::Output>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands, Command};

    #[test]
    fn can_create_new_client_and_issue_command() {
        let mut client = Client::new("localhost:6379").unwrap();
        dbg!(client.issue_command(Command::cmd("GET").with_arg("mykey")));
        dbg!(client.issue_command(Command::cmd("PRINTLN")));
        dbg!(client.issue_command(commands::set("my-test-key", 32)));
        // TODO: tidy these generics up
        dbg!(client.issue_command::<commands::Get<Option<i64>>>(commands::get("my-test-key")));
    }
}
