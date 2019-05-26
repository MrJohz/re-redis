use crate::sans_io::SansIoClient;
use crate::{Command, RedisError, RedisValue};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use void::Void;

pub struct Client {
    writer: BufWriter<TcpStream>,
    parser: SansIoClient,
}

impl Client {
    pub fn new(address: impl ToSocketAddrs) -> Self {
        let stream = TcpStream::connect(address).unwrap();
        let writer = BufWriter::new(stream.try_clone().unwrap());
        let (parser, tx_bytes) = SansIoClient::new();

        thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            let mut buffer = String::new();

            while let Ok(_) = reader.read_line(&mut buffer) {
                tx_bytes.send(buffer.clone().into());
                &buffer.clear();
            }

            // TODO: figure out why this closes when the tests are finished
        });

        Self { parser, writer }
    }

    pub fn issue_command(&mut self, cmd: Command) -> Result<Option<RedisValue>, RedisError<Void>> {
        let bytes = self.parser.issue_command(cmd);
        self.writer.write(&bytes);
        self.writer.flush();
        self.parser.get_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Command;

    #[test]
    fn can_create_new_client_and_issue_command() {
        let mut client = Client::new("localhost:6379");
        dbg!(client.issue_command(Command::cmd("GET").with_arg("mykey")));
        dbg!(client.issue_command(Command::cmd("PRINTLN")));
    }
}
