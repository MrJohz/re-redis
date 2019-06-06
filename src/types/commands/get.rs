use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;

use crate::types::redis_values::{ConversionError, RedisResult};
use crate::types::StructuredCommand;
use crate::utils::validate_key;

pub struct Get<T> {
    key: String,
    _t: PhantomData<T>,
}

impl<T> Get<T> {
    pub(self) fn new(key: String) -> Self {
        Self {
            key,
            _t: PhantomData,
        }
    }

    pub fn with_default(self, default: T) -> GetWithDefault<T> {
        GetWithDefault {
            get_command: self,
            default,
        }
    }
}

impl<T> StructuredCommand for Get<T>
where
    RedisResult: TryInto<Option<T>, Error = ConversionError>,
{
    type Output = Option<T>;

    fn get_bytes(&self) -> Vec<u8> {
        format!(
            "*2\r\n\
             $3\r\nGET\r\n\
             ${}\r\n{}\r\n",
            self.key.len(),
            self.key
        )
        .into()
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        result.try_into()
    }
}

pub struct GetWithDefault<T> {
    get_command: Get<T>,
    default: T,
}

impl<T> StructuredCommand for GetWithDefault<T>
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

pub fn get<T>(key: String) -> Get<T> {
    Get::new(validate_key(key))
}

pub struct GetMultipleList<T> {
    keys: Vec<String>,
    _t: PhantomData<T>,
}

impl<T> GetMultipleList<T> {
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.keys.push(key.into());
        self
    }

    pub fn with_keys(mut self, keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keys = keys.into_iter().map(Into::into).collect();
        self
    }
}

impl<T> StructuredCommand for GetMultipleList<T>
where
    RedisResult: TryInto<Option<T>, Error = ConversionError>,
{
    type Output = Vec<Option<T>>;

    fn get_bytes(&self) -> Vec<u8> {
        let mut message = format!(
            "*{msg_size}\r\n\
             $4\r\nMGET\r\n",
            msg_size = self.keys.len() + 1,
        );

        for key in &self.keys {
            message.push_str(&format!("${len}\r\n{key}\r\n", len = key.len(), key = key))
        }

        message.into()
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

pub fn mget<T>() -> GetMultipleList<T> {
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
        let cmd = get::<isize>("test".into());

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "*2\r\n\
             $3\r\nGET\r\n\
             $4\r\ntest\r\n"
        )
    }
}
