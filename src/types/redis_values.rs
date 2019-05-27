use std::convert::TryFrom;
use std::error::Error;

#[derive(Debug, Eq, PartialEq)]
pub struct RedisErrorValue {
    contents: String,
}

impl RedisErrorValue {
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
pub enum RedisResult {
    String(String),
    Integer(i64),
    Error(RedisErrorValue),
    Array(Vec<RedisResult>),
    Null,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RedisValue {
    String(String),
    Integer(i64),
    Array(Vec<Option<RedisValue>>),
}

#[derive(Debug)]
pub enum ConversionError {
    NoConversionTypeMatch { value: Option<RedisValue> },
    RedisReturnedError { error: RedisErrorValue },
    CannotParseStringResponse { error: Box<Error> },
}

macro_rules! create_try_from_impl {
    ($destination:ty; $value:ident => {
        $($pattern:pat => $result:expr,)+
    }) => {
        impl TryFrom<RedisResult> for $destination {
            type Error = ConversionError;

            fn try_from($value: RedisResult) -> Result<Self, Self::Error> {
                match $value {
                    $($pattern => $result,)*
                    RedisResult::Error(error) => Err(ConversionError::RedisReturnedError { error }),
                    _ => Err(ConversionError::NoConversionTypeMatch { value: Option::try_from($value).unwrap() }),
                }
            }
        }
    };
}

impl TryFrom<RedisResult> for Option<RedisValue> {
    type Error = ConversionError;

    fn try_from(r: RedisResult) -> Result<Self, Self::Error> {
        match r {
            RedisResult::String(string) => Ok(Some(RedisValue::String(string))),
            RedisResult::Integer(int) => Ok(Some(RedisValue::Integer(int))),
            RedisResult::Array(array) => Ok(Some(RedisValue::Array(
                array
                    .into_iter()
                    .map(Option::try_from)
                    .collect::<Result<_, _>>()?,
            ))),
            RedisResult::Null => Ok(None),
            RedisResult::Error(error) => Err(ConversionError::RedisReturnedError { error }),
        }
    }
}

impl TryFrom<RedisResult> for () {
    type Error = ConversionError;

    fn try_from(r: RedisResult) -> Result<Self, Self::Error> {
        match r {
            RedisResult::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Ok(()),
        }
    }
}

create_try_from_impl! { Option<String>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::String(text) => Ok(Some(text)),
}}

create_try_from_impl! { Option<isize>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int as isize)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<i64>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<i32>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int as i32)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<i16>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int as i16)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<i8>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int as i8)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<usize>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int as usize)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<u64>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int as u64)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<u32>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int as u32)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<u16>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int as u16)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<u8>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int as u8)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<f64>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int as f64)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

create_try_from_impl! { Option<f32>; value => {
    RedisResult::Null => Ok(None),
    RedisResult::Integer(int) => Ok(Some(int as f32)),
    RedisResult::String(text) => Ok(Some(text
        .parse()
        .map_err(|err| ConversionError::CannotParseStringResponse { error: Box::new(err) })?
    )),
}}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redis_error_contains_correct_two_parts() {
        let error = RedisErrorValue::new("TEST This tests that the error struct works");

        assert_eq!(Some("TEST"), error.kind());
        assert_eq!(
            Some("This tests that the error struct works"),
            error.message()
        )
    }
}
