use std::convert::TryFrom;

// TODO: create an error enum that can be used here

#[derive(Debug, PartialEq, Eq)]
pub enum RedisValue {
    String(String),
    Integer(i64),
    Array(Vec<RedisValue>),
    Error(String), // use a string for now, can be changed to a unique RedisError enum
    Null,
}

impl TryFrom<RedisValue> for () {
    type Error = String;

    fn try_from(r: RedisValue) -> Result<Self, Self::Error> {
        match r {
            RedisValue::Error(text) => Err(text.to_string()),
            _ => Ok(()),
        }
    }
}

impl TryFrom<RedisValue> for i64 {
    type Error = String;

    fn try_from(r: RedisValue) -> Result<Self, Self::Error> {
        match r {
            RedisValue::Integer(int) => Ok(int),
            RedisValue::Error(text) => Err(text.to_string()),
            _ => Err("Invalid response type".to_string()),
        }
    }
}

impl TryFrom<RedisValue> for String {
    type Error = String;

    fn try_from(r: RedisValue) -> Result<Self, Self::Error> {
        match r {
            RedisValue::String(text) => Ok(text.to_string()),
            RedisValue::Error(text) => Err(text.to_string()),
            _ => Err("Invalid response type".to_string()),
        }
    }
}

impl TryFrom<RedisValue> for bool {
    type Error = String;

    fn try_from(r: RedisValue) -> Result<Self, Self::Error> {
        match r {
            RedisValue::Integer(int) => Ok(int > 0),
            RedisValue::Error(text) => Err(text.to_string()),
            _ => Err("Invalid response type".to_string()),
        }
    }
}
