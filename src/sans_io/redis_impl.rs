use crate::sans_io::response_parser::{ParseError, ResponseParser};
use crate::{Command, RedisError, RedisValue};
use std::convert::TryInto;
use std::io::Result as IoResult;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug, PartialEq, Eq)]
pub enum RedisSansEvent {
    BytesSent(Vec<u8>),
    ServerResponseReceived(RedisValue),
    ProtocolError(ParseError),
}

#[derive(Debug)]
pub struct Client {
    has_errored: bool,
    has_finished: bool,
    receive_bytes: Receiver<IoResult<Vec<u8>>>,
    parser: ResponseParser,
}

impl Client {
    pub fn new() -> (Self, Sender<IoResult<Vec<u8>>>) {
        let (tx_bytes, rx_bytes) = channel();
        (
            Self {
                has_finished: false,
                has_errored: false,
                receive_bytes: rx_bytes,
                parser: ResponseParser::new(),
            },
            tx_bytes,
        )
    }

    pub fn issue_command(&self, cmd: Command) -> Vec<u8> {
        if self.has_finished {
            return Vec::new();
        }

        cmd.get_command_string()
    }

    pub fn get_response(&mut self) -> Result<Option<RedisValue>, RedisError> {
        loop {
            match self.parser.get_response() {
                Ok(Some(value)) => return value.try_into().map_err(RedisError::RedisReturnedError),
                Err(error) => return Err(RedisError::ProtocolParseError(error)),
                Ok(None) => {
                    let bytes = self
                        .receive_bytes
                        .recv()
                        .map_err(RedisError::InternalConnectionError)?
                        .map_err(RedisError::ConnectionError)?;
                    self.parser.feed(&bytes);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: reimplement these tests

    //    use super::*;
    //    use crate::{Command, RedisValue};
    //    use std::sync::mpsc::channel;

    //    #[test]
    //    fn bytes_sent_handler_is_called_when_command_is_issed() {
    //        let (tx, rx) = channel();
    //        let (_ignored, _) = channel();
    //
    //        let mut client = Client::new(_ignored, tx);
    //        client.issue_command(Command::cmd("GET").with_arg("my_favourite_key"));
    //
    //        assert_eq!(Ok(Vec::from("GET my_favourite_key")), rx.try_recv(),);
    //    }

    //    #[test]
    //    fn server_response_received_when_redis_value_is_parsed() {
    //        let (tx, rx) = channel();
    //
    //        let mut client = Client::new(tx);
    //        client.feed_data(":42\r\n".as_bytes());
    //
    //        assert_eq!(Ok(Ok(Some(RedisValue::Integer(42)))), rx.try_recv(),);
    //    }
}
