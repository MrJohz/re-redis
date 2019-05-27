use crate::sans_io::SansIoClient;
use crate::{Command, RedisError, RedisValue};
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

    pub fn issue_command(&mut self, cmd: Command) -> Result<Option<RedisValue>, RedisError> {
        let bytes = self.parser.issue_command(cmd);
        self.writer
            .write(&bytes)
            .map_err(RedisError::ConnectionError)?;
        self.writer.flush().map_err(RedisError::ConnectionError)?;
        self.parser.get_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Command;

    #[test]
    fn can_create_new_client_and_issue_command() {
        let mut client = Client::new("localhost:6379").unwrap();
        dbg!(client.issue_command(Command::cmd("GET").with_arg("mykey")));
        dbg!(client.issue_command(Command::cmd("PRINTLN")));
    }
}
