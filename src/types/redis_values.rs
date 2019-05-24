use std::convert::TryFrom;
use std::error::Error;

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
        self.contents.splitn(2, ' ').nth(1)
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
    CannotParseStringResponse { error: Box<Error> },
}

macro_rules! create_try_from_impl {
    ($destination:ty; $value:ident => {
        $($pattern:pat => $result:expr,)+
    }) => {
        impl TryFrom<RedisValue> for $destination {
            type Error = ConversionError;

            fn try_from($value: RedisValue) -> Result<Self, Self::Error> {
                match $value {
                    $($pattern => $result,)*
                    RedisValue::Error(error) => Err(ConversionError::RedisReturnedError { error }),
                    _ => Err(ConversionError::NoConversionTypeMatch { value: $value }),
                }
            }
        }
    };
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

create_try_from_impl! { Option<String>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::String(text) => Ok(Some(text)),
}}

create_try_from_impl! { Option<isize>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int as isize)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<i64>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<i32>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int as i32)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<i16>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int as i16)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<i8>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int as i8)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<usize>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int as usize)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<u64>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int as u64)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<u32>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int as u32)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<u16>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int as u16)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<u8>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int as u8)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<f64>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int as f64)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<f32>; value => {
    RedisValue::Null => Ok(None),
    RedisValue::Integer(int) => Ok(Some(int as f32)),
    RedisValue::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

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
