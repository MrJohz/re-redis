use crate::sans_io::ParseError;
use crate::{RedisErrorValue, RedisValue};
use std::error::Error;
use std::io::Error as IoError;
use std::sync::mpsc::RecvError;

#[derive(Debug)]
pub enum RedisError {
    ConnectionError(IoError),
    RedisReturnedError(RedisErrorValue),
    ProtocolParseError(ParseError),
    InternalConnectionError(RecvError),
    ConversionError(Option<RedisValue>),
    StringParseError(Box<Error>),
}
