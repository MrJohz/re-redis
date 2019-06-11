use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;

use crate::types::redis_values::{ConversionError, RedisResult};
use crate::types::StructuredCommand;
use crate::RBytes;

pub struct Get<'a, T> {
    key: RBytes<'a>,
    _t: PhantomData<T>,
}

impl<'a, T> Get<'a, T> {
    pub(self) fn new(key: RBytes<'a>) -> Self {
        Self {
            key,
            _t: PhantomData,
        }
    }

    pub fn with_default(self, default: T) -> GetWithDefault<'a, T> {
        GetWithDefault {
            get_command: self,
            default,
        }
    }
}

impl<'a, T> StructuredCommand for Get<'a, T>
where
    RedisResult: TryInto<Option<T>, Error = ConversionError>,
{
    type Output = Option<T>;

    fn get_bytes(&self) -> Vec<u8> {
        resp_bytes!("GET", &self.key)
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        result.try_into()
    }
}

pub struct GetWithDefault<'a, T> {
    get_command: Get<'a, T>,
    default: T,
}

impl<'a, T> StructuredCommand for GetWithDefault<'a, T>
where
    RedisResult: TryInto<Option<T>, Error = ConversionError>,
{
    type Output = T;

    fn get_bytes(&self) -> Vec<u8> {
        self.get_command.get_bytes()
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        let intermediate: Result<Option<Self::Output>, ConversionError> = result.try_into();
        intermediate.map(|option| option.unwrap_or(self.default))
    }
}

pub fn get<'a, T, B>(key: B) -> Get<'a, T>
where
    B: Into<RBytes<'a>>,
{
    Get::new(key.into())
}

pub struct GetMultipleList<'a, T> {
    keys: Vec<RBytes<'a>>,
    _t: PhantomData<T>,
}

impl<'a, T> GetMultipleList<'a, T> {
    pub fn key(mut self, key: impl Into<RBytes<'a>>) -> Self {
        self.keys.push(key.into());
        self
    }

    pub fn with_keys(mut self, keys: impl IntoIterator<Item = impl Into<RBytes<'a>>>) -> Self {
        self.keys = keys.into_iter().map(Into::into).collect();
        self
    }
}

impl<'a, T> StructuredCommand for GetMultipleList<'a, T>
where
    RedisResult: TryInto<Option<T>, Error = ConversionError>,
{
    type Output = Vec<Option<T>>;

    fn get_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.push(b'*');
        message.extend_from_slice((self.keys.len() + 1).to_string().as_bytes());
        message.extend_from_slice(b"\r\n$4\r\nMGET\r\n");

        for key in &self.keys {
            insert_bytes_into_vec!(message, key);
        }

        message
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        match result {
            RedisResult::Array(results) => results.into_iter().map(|r| r.try_into()).collect(),
            RedisResult::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Err(ConversionError::NoConversionTypeMatch {
                value: Option::try_from(result)?,
            }),
        }
    }
}

pub fn mget<'a, T>() -> GetMultipleList<'a, T> {
    GetMultipleList {
        keys: Vec::new(),
        _t: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_command_converts_to_bytes_with_correct_type_specification() {
        let cmd = get::<isize, _>("test");

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "*2\r\n\
             $3\r\nGET\r\n\
             $4\r\ntest\r\n"
        )
    }
}
