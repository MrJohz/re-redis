use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;
use std::time::Duration;

use crate::types::redis_values::{ConversionError, RedisResult};
use crate::types::StructuredCommand;
use crate::RBytes;

pub struct Set<'a> {
    key: RBytes<'a>,
    value: RBytes<'a>,
    expiry: Option<Duration>,
}

impl<'a> Set<'a> {
    pub(self) fn new(key: RBytes<'a>, value: RBytes<'a>) -> Self {
        Self {
            key,
            value,
            expiry: None,
        }
    }

    pub fn with_expiry(mut self, duration: Duration) -> Self {
        if cfg!(debug_assertions) && duration == Duration::from_millis(0) {
            panic!("duration cannot be 0 in length");
        }

        self.expiry.replace(duration);
        self
    }

    pub fn if_exists(self) -> SetIfExists<'a> {
        SetIfExists {
            key: self.key,
            expiry: self.expiry,
            value: self.value,
            exists: true,
        }
    }

    pub fn if_not_exists(self) -> SetIfExists<'a> {
        SetIfExists {
            key: self.key,
            expiry: self.expiry,
            value: self.value,
            exists: false,
        }
    }
}

pub struct SetIfExists<'a> {
    key: RBytes<'a>,
    value: RBytes<'a>,
    expiry: Option<Duration>,
    exists: bool,
}

impl<'a> SetIfExists<'a> {
    pub fn with_expiry(mut self, duration: Duration) -> Self {
        if cfg!(debug_assertions) && duration == Duration::from_millis(0) {
            panic!("duration cannot be 0 in length");
        }

        self.expiry.replace(duration);
        self
    }
}

pub fn set<'a>(key: impl Into<RBytes<'a>>, value: impl Into<RBytes<'a>>) -> Set<'a> {
    Set::new(key.into(), value.into())
}

impl<'a> StructuredCommand for Set<'a> {
    type Output = ();

    fn get_bytes(&self) -> Vec<u8> {
        match self.expiry {
            Some(duration) => resp_bytes!(
                "SET",
                self.key,
                self.value,
                "PX",
                duration.as_millis().to_string()
            ),
            None => resp_bytes!("SET", self.key, self.value),
        }
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        result.try_into()
    }
}

impl<'a> StructuredCommand for SetIfExists<'a> {
    type Output = bool;

    fn get_bytes(&self) -> Vec<u8> {
        let exists_tag = if self.exists { "XX" } else { "NX" };
        match self.expiry {
            Some(duration) => resp_bytes!(
                "SET",
                self.key,
                self.value,
                "PX",
                duration.as_millis().to_string(),
                exists_tag
            ),
            None => resp_bytes!("SET", self.key, self.value, exists_tag),
        }
        .into()
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        match result {
            RedisResult::Null => Ok(false),
            RedisResult::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Ok(true),
        }
    }
}

pub struct SetMany<'a> {
    key_value_pairs: Vec<(RBytes<'a>, RBytes<'a>)>,
}

impl<'a> SetMany<'a> {
    pub fn add(mut self, key: impl Into<RBytes<'a>>, value: impl Into<RBytes<'a>>) -> Self {
        self.key_value_pairs.push((key.into(), value.into()));
        self
    }

    pub fn with_pairs(
        mut self,
        pairs: impl IntoIterator<Item = (impl Into<RBytes<'a>>, impl Into<RBytes<'a>>)>,
    ) -> Self {
        self.key_value_pairs = pairs
            .into_iter()
            .map(|(first, second)| (first.into(), second.into()))
            .collect();
        self
    }

    pub fn if_none_exists(self) -> SetManyIfExists<'a> {
        SetManyIfExists {
            key_value_pairs: self.key_value_pairs,
        }
    }
}

impl<'a> StructuredCommand for SetMany<'a> {
    type Output = ();

    fn get_bytes(&self) -> Vec<u8> {
        let message_size = ((self.key_value_pairs.len() * 2) + 1).to_string();

        let mut bytes = Vec::new();

        bytes.push('*' as u8);
        bytes.extend_from_slice(message_size.as_bytes());
        bytes.extend_from_slice("\r\n$4\r\nMSET\r\n".as_bytes());

        for (key, value) in &self.key_value_pairs {
            insert_bytes_into_vec!(bytes, key);
            insert_bytes_into_vec!(bytes, value);
        }

        bytes
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        result.try_into()
    }
}

pub struct SetManyIfExists<'a> {
    key_value_pairs: Vec<(RBytes<'a>, RBytes<'a>)>,
}

impl<'a> StructuredCommand for SetManyIfExists<'a> {
    type Output = bool;

    fn get_bytes(&self) -> Vec<u8> {
        let message_size = ((self.key_value_pairs.len() * 2) + 1).to_string();

        let mut bytes = Vec::new();

        bytes.push('*' as u8);
        bytes.extend_from_slice(message_size.as_bytes());
        bytes.extend_from_slice("\r\n$6\r\nMSETNX\r\n".as_bytes());

        for (key, value) in &self.key_value_pairs {
            insert_bytes_into_vec!(bytes, key);
            insert_bytes_into_vec!(bytes, value);
        }

        bytes
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        match result {
            RedisResult::Integer(0) => Ok(false),
            RedisResult::Integer(1) => Ok(true),
            RedisResult::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Err(ConversionError::NoConversionTypeMatch {
                value: Option::try_from(result)?,
            }),
        }
    }
}

pub fn mset<'a>() -> SetMany<'a> {
    SetMany {
        key_value_pairs: Vec::new(),
    }
}

pub struct GetSet<'a, T> {
    key: RBytes<'a>,
    value: RBytes<'a>,
    _t: PhantomData<T>,
}

impl<'a, T> StructuredCommand for GetSet<'a, T>
where
    RedisResult: TryInto<Option<T>, Error = ConversionError>,
{
    type Output = Option<T>;

    fn get_bytes(&self) -> Vec<u8> {
        resp_bytes!("GETSET", self.key, self.value)
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        result.try_into()
    }
}

pub fn getset<'a, T, K, V>(key: K, value: V) -> GetSet<'a, T>
where
    K: Into<RBytes<'a>>,
    V: Into<RBytes<'a>>,
{
    GetSet {
        key: key.into(),
        value: value.into(),
        _t: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_command_converts_to_bytes_with_expiry_data() {
        let cmd = set("my-first-key", 42).with_expiry(Duration::from_secs(400));

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "*5\r\n\
             $3\r\nSET\r\n\
             $12\r\nmy-first-key\r\n\
             $2\r\n42\r\n\
             $2\r\nPX\r\n\
             $6\r\n400000\r\n"
        );
    }

    #[test]
    fn set_command_can_transform_to_if_exists_format() {
        let cmd = set("my-first-key", 42).if_exists();

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "*4\r\n\
             $3\r\nSET\r\n\
             $12\r\nmy-first-key\r\n\
             $2\r\n42\r\n\
             $2\r\nXX\r\n"
        );
    }

    #[test]
    fn set_if_exists_can_have_an_optional_duration() {
        let cmd = set("my-first-key", 42)
            .if_exists()
            .with_expiry(Duration::from_millis(1000));

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "*6\r\n\
             $3\r\nSET\r\n\
             $12\r\nmy-first-key\r\n\
             $2\r\n42\r\n\
             $2\r\nPX\r\n\
             $4\r\n1000\r\n\
             $2\r\nXX\r\n"
        );
    }

    #[test]
    fn set_command_with_duration_keeps_duration_when_transformed_to_set_if_exists() {
        let cmd = set("my-first-key", 42)
            .with_expiry(Duration::from_millis(1000))
            .if_exists();

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "*6\r\n\
             $3\r\nSET\r\n\
             $12\r\nmy-first-key\r\n\
             $2\r\n42\r\n\
             $2\r\nPX\r\n\
             $4\r\n1000\r\n\
             $2\r\nXX\r\n"
        );
    }

    #[test]
    fn set_command_can_transform_to_if_not_exists_format() {
        let cmd = set("my-first-key", 42).if_not_exists();

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "*4\r\n\
             $3\r\nSET\r\n\
             $12\r\nmy-first-key\r\n\
             $2\r\n42\r\n\
             $2\r\nNX\r\n"
        );
    }
}
