use crate::redis_values::ConversionError;
use crate::sans_io::response_parser::{ParseError, ResponseParser};
use crate::{RedisError, RedisResult, RedisValue, StructuredCommand};
use std::convert::{TryFrom, TryInto};
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

    pub fn issue_command(&self, cmd: impl StructuredCommand) -> Vec<u8> {
        if self.has_finished {
            return Vec::new();
        }

        cmd.into_bytes()
    }

    pub fn get_response<T>(&mut self) -> Result<T, RedisError>
    where
        T: TryFrom<RedisResult, Error = ConversionError>,
    {
        loop {
            match self.parser.get_response() {
                Ok(Some(value)) => {
                    return value.try_into().map_err(|err| match err {
                        ConversionError::NoConversionTypeMatch { value } => {
                            RedisError::ConversionError(value)
                        }
                        ConversionError::RedisReturnedError { error } => {
                            RedisError::RedisReturnedError(error)
                        }
                        ConversionError::CannotParseStringResponse { error } => {
                            RedisError::StringParseError(error)
                        }
                    })
                }
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

//#[cfg(test)]
//mod tests {
//    use super::*;
//    use crate::{Command, RedisValue};
//
//    #[test]
//    fn bytes_sent_handler_is_called_when_command_is_issed() {
//        let (client, _) = Client::new();
//        let bytes = client.issue_command(Command::cmd("GET").with_arg("my_favourite_key"));
//        assert_eq!("GET my_favourite_key\r\n".as_bytes(), bytes.as_slice());
//    }
//
//    #[test]
//    fn server_response_received_when_redis_value_is_parsed() {
//        let (mut client, send_bytes) = Client::new();
//        send_bytes.send(Ok(":42\r\n".into())).unwrap();
//
//        assert_eq!(
//            Some(RedisValue::Integer(42)),
//            client.get_response().unwrap()
//        );
//    }
//}
