use std::convert::TryFrom;

// TODO: create an error enum that can be used here

#[derive(Debug, Eq, PartialEq)]
pub struct RedisError {
    contents: String,
}

impl RedisError {
    pub(crate) fn new(contents: impl Into<String>) -> Self {
        Self {
            contents: contents.into(),
        }
    }

    pub fn kind(&self) -> Option<&str> {
        self.contents.splitn(2, ' ').next()
    }

    pub fn message(&self) -> Option<&str> {
        self.contents.splitn(2, ' ').skip(1).next()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RedisValue {
    String(String),
    Integer(i64),
    Array(Vec<RedisValue>),
    Error(RedisError), // use a string for now, can be changed to a unique RedisError enum
    Null,
}

pub enum ConversionError {
    NoConversionTypeMatch { value: RedisValue },
    RedisReturnedError { error: RedisError },
}

impl TryFrom<RedisValue> for () {
    type Error = ConversionError;

    fn try_from(r: RedisValue) -> Result<Self, Self::Error> {
        match r {
            RedisValue::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Ok(()),
        }
    }
}

impl TryFrom<RedisValue> for i64 {
    type Error = ConversionError;

    fn try_from(value: RedisValue) -> Result<Self, Self::Error> {
        match value {
            RedisValue::Integer(int) => Ok(int),
            RedisValue::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Err(ConversionError::NoConversionTypeMatch { value }),
        }
    }
}

impl TryFrom<RedisValue> for String {
    type Error = ConversionError;

    fn try_from(value: RedisValue) -> Result<Self, Self::Error> {
        match value {
            RedisValue::String(text) => Ok(text.to_string()),
            RedisValue::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Err(ConversionError::NoConversionTypeMatch { value }),
        }
    }
}

impl TryFrom<RedisValue> for bool {
    type Error = ConversionError;

    fn try_from(value: RedisValue) -> Result<Self, Self::Error> {
        match value {
            RedisValue::Integer(int) => Ok(int > 0),
            RedisValue::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Err(ConversionError::NoConversionTypeMatch { value }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redis_error_contains_correct_two_parts() {
        let error = RedisError::new("TEST This tests that the error struct works");

        assert_eq!(Some("TEST"), error.kind());
        assert_eq!(
            Some("This tests that the error struct works"),
            error.message()
        )
    }
}
