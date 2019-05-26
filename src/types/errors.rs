use crate::sans_io::ParseError;
use crate::RedisErrorValue;
use std::error::Error;
use std::sync::mpsc::RecvError;

#[derive(Debug, PartialEq, Eq)]
pub enum RedisError<T: Error> {
    ConnectionError(T),
    RedisReturnedError(RedisErrorValue),
    ProtocolParseError(ParseError),
    InternalConnectionError(RecvError),
}
