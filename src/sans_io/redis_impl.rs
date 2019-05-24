use crate::sans_io::response_parser::{ParseError, ResponseParser};
use crate::{Command, RedisValue};

pub enum RedisSansEvent {
    BytesSent(Vec<u8>),
    ServerResponseReceived(RedisValue),
    ProtocolError(ParseError),
}

pub struct Redis<CB: FnMut(RedisSansEvent) -> ()> {
    pub(self) has_errored: bool,
    pub(self) handler: CB,
    pub(self) parser: ResponseParser,
}

impl<CB: FnMut(RedisSansEvent) -> ()> Redis<CB> {
    pub fn new(handler: CB) -> Self {
        Self {
            handler,
            has_errored: false,
            parser: ResponseParser::new(),
        }
    }

    pub fn issue_command(&mut self, cmd: Command) {
        (self.handler)(RedisSansEvent::BytesSent(cmd.get_command_string()));
    }

    pub fn feed_data(&mut self, bytes: &[u8]) {
        if self.has_errored {
            return;
        }

        self.parser.feed(bytes);
        loop {
            match self.parser.get_response() {
                Ok(Some(value)) => (self.handler)(RedisSansEvent::ServerResponseReceived(value)),
                Ok(None) => break,
                Err(error) => {
                    self.has_errored = true;
                    (self.handler)(RedisSansEvent::ProtocolError(error));
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, RedisValue};

    #[test]
    fn bytes_sent_handler_is_called_when_command_is_issed() {
        let mut handler_has_been_called = false;

        let mut client = Redis::new(|event| {
            handler_has_been_called = true;
            match event {
                RedisSansEvent::BytesSent(bytes) => {
                    assert_eq!(Vec::from("GET my_favourite_key"), bytes)
                }
                _ => assert!(false, "handler called with the wrong event type"),
            }
        });

        client.issue_command(Command::cmd("GET").with_arg("my_favourite_key"));

        assert_eq!(handler_has_been_called, true);
    }

    #[test]
    fn server_response_received_when_redis_value_is_parsed() {
        let mut handler_has_been_called = false;

        let mut client = Redis::new(|event| {
            handler_has_been_called = true;
            match event {
                RedisSansEvent::ServerResponseReceived(value) => {
                    assert_eq!(RedisValue::Integer(42), value);
                }
                _ => assert!(false, "handler called with the wrong event type"),
            }
        });

        client.feed_data(":42\r\n".as_bytes());

        assert_eq!(handler_has_been_called, true);
    }
}
