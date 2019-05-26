use crate::sans_io::SansIoClient;
use crate::{Command, RedisError, RedisValue};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::thread::{JoinHandle, Thread};
use std::time::Duration;
use void::Void;

pub struct Client {
    receive_values: Receiver<Result<Option<RedisValue>, RedisError<Void>>>,
    parser: SansIoClient,
}

impl Client {
    pub fn new(address: impl ToSocketAddrs) -> Self {
        // TODO: CondVar might be a better choice here, but I am not 100% certain
        let (tx_redis_values, rx_redis_values) = channel();
        let (tx_bytes, rx_bytes) = channel();

        let mut stream = TcpStream::connect(address).unwrap();
        let mut parser = SansIoClient::new(tx_redis_values, tx_bytes);

        thread::spawn(move || {
            while let Ok(bytes) = rx_bytes.recv() {
                dbg!(bytes);
            }

            // automatically close this thread when the bytes transfer channel dies,
            // which will be when the parser gets dropped, which will be when the client
            // is dropped.  Lovely, lovely drop semantics.
        });

        parser.feed_data(":43\r\n".as_bytes());
        parser.feed_data("+52\r\n".as_bytes());

        Self {
            parser,
            receive_values: rx_redis_values,
        }
    }

    pub fn issue_command(&mut self, cmd: Command) -> Result<Option<RedisValue>, RedisError<Void>> {
        self.parser.issue_command(cmd);
        match self
            .receive_values
            .recv()
            .map_err(RedisError::InternalConnectionError)
        {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) => Err(error),
            Err(error) => Err(error), // TODO: I don't think this is a possible state to be in
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Command;

    #[test]
    fn can_create_new_client_and_issue_command() {
        let mut client = Client::new("localhost:6379");
        dbg!(client.issue_command(Command::cmd("GET")));
        dbg!(client.issue_command(Command::cmd("PRINTLN")));
    }
}
