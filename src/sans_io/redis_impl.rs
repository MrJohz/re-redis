use crate::sans_io::response_parser::{ParseError, ResponseParser};
use crate::{Command, RedisError, RedisValue};
use std::convert::TryInto;
use std::sync::mpsc::Sender;
use void::Void; // TODO: replace this with the ! type when it's stabilised

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
    send_events: Sender<Result<Option<RedisValue>, RedisError<Void>>>,
    send_bytes: Sender<Vec<u8>>,
    parser: ResponseParser,
}

impl Client {
    // TODO: abstract Sender out into some sort of trait
    pub fn new(
        send_events: Sender<Result<Option<RedisValue>, RedisError<Void>>>,
        send_bytes: Sender<Vec<u8>>,
    ) -> Self {
        Self {
            send_events,
            send_bytes,
            has_finished: false,
            has_errored: false,
            parser: ResponseParser::new(),
        }
    }

    pub fn issue_command(&mut self, cmd: Command) {
        if self.has_finished {
            return;
        }

        self.send_bytes
            .send(cmd.get_command_string())
            .unwrap_or_else(|_| self.has_finished = true);
    }

    pub fn feed_data(&mut self, bytes: &[u8]) {
        if self.has_errored || self.has_finished {
            return;
        }

        self.parser.feed(bytes);
        loop {
            match self.parser.get_response() {
                Ok(Some(value)) => self
                    .send_events
                    .send(value.try_into().map_err(RedisError::RedisReturnedError))
                    .unwrap_or_else(|_| self.has_finished = true),
                Ok(None) => break,
                Err(error) => {
                    self.has_errored = true;
                    self.send_events
                        .send(Err(RedisError::ProtocolParseError(error)))
                        .unwrap_or_else(|_| self.has_finished = true); // just straight up ignore this error
                    break;
                }
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, RedisValue};
    use std::sync::mpsc::channel;

    #[test]
    fn bytes_sent_handler_is_called_when_command_is_issed() {
        let (tx, rx) = channel();
        let (_ignored, _) = channel();

        let mut client = Client::new(_ignored, tx);
        client.issue_command(Command::cmd("GET").with_arg("my_favourite_key"));

        assert_eq!(Ok(Vec::from("GET my_favourite_key")), rx.try_recv(),);
    }

    #[test]
    fn server_response_received_when_redis_value_is_parsed() {
        let (tx, rx) = channel();
        let (_ignored, _) = channel();

        let mut client = Client::new(tx, _ignored);
        client.feed_data(":42\r\n".as_bytes());

        assert_eq!(Ok(Ok(Some(RedisValue::Integer(42)))), rx.try_recv(),);
    }
}
