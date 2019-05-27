use crate::sans_io::ParseError;
use crate::RedisErrorValue;
use std::io::Error as IoError;
use std::sync::mpsc::RecvError;

#[derive(Debug)]
pub enum RedisError {
    ConnectionError(IoError),
    RedisReturnedError(RedisErrorValue),
    ProtocolParseError(ParseError),
    InternalConnectionError(RecvError),
}
