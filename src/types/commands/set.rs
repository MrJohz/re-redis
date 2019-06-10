use std::convert::{TryFrom, TryInto};
use std::time::Duration;

use crate::types::command::RedisArg;
use crate::types::redis_values::{ConversionError, RedisResult};
use crate::types::StructuredCommand;
use crate::utils::validate_key;
use std::marker::PhantomData;

pub struct Set {
    key: String,
    expiry: Option<Duration>,
    value: String,
}

impl Set {
    pub(self) fn new(key: String, value: impl Into<RedisArg>) -> Self {
        Self {
            key,
            expiry: None,
            value: value.into().0,
        }
    }

    pub fn with_expiry(mut self, duration: Duration) -> Self {
        self.expiry.replace(duration);
        self
    }

    pub fn if_exists(self) -> SetIfExists {
        SetIfExists {
            key: self.key,
            expiry: self.expiry,
            value: self.value,
            exists: true,
        }
    }

    pub fn if_not_exists(self) -> SetIfExists {
        SetIfExists {
            key: self.key,
            expiry: self.expiry,
            value: self.value,
            exists: false,
        }
    }
}

pub struct SetIfExists {
    key: String,
    expiry: Option<Duration>,
    value: String,
    exists: bool,
}

impl SetIfExists {
    pub fn with_expiry(mut self, duration: Duration) -> Self {
        if cfg!(debug_assertions) && duration == Duration::from_millis(0) {
            panic!("duration cannot be 0 in length");
        }

        self.expiry.replace(duration);
        self
    }
}

pub fn set(key: impl Into<String>, value: impl Into<RedisArg>) -> Set {
    Set::new(validate_key(key), value)
}

impl StructuredCommand for Set {
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

impl StructuredCommand for SetIfExists {
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

pub struct SetMany {
    key_value_pairs: Vec<(String, String)>,
}

impl SetMany {
    pub fn add(mut self, key: impl Into<String>, value: impl Into<RedisArg>) -> Self {
        self.key_value_pairs.push((key.into(), value.into().0));
        self
    }

    pub fn with_pairs(
        mut self,
        pairs: impl IntoIterator<Item = (impl Into<String>, impl Into<RedisArg>)>,
    ) -> Self {
        self.key_value_pairs = pairs
            .into_iter()
            .map(|(first, second)| (first.into(), second.into().0))
            .collect();
        self
    }

    pub fn if_none_exists(self) -> SetManyIfExists {
        SetManyIfExists {
            key_value_pairs: self.key_value_pairs,
        }
    }
}

impl StructuredCommand for SetMany {
    type Output = ();

    fn get_bytes(&self) -> Vec<u8> {
        let mut header = format!(
            "*{msg_size}\r\n\
             $4\r\nMSET\r\n",
            msg_size = (self.key_value_pairs.len() * 2) + 1,
        );

        for (key, value) in &self.key_value_pairs {
            header.push_str(&format!(
                "${key_length}\r\n{key}\r\n\
                 ${value_length}\r\n{value}\r\n",
                key = key,
                key_length = key.len(),
                value = value,
                value_length = value.len(),
            ));
        }

        header.into()
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        result.try_into()
    }
}

pub struct SetManyIfExists {
    key_value_pairs: Vec<(String, String)>,
}

impl StructuredCommand for SetManyIfExists {
    type Output = bool;

    fn get_bytes(&self) -> Vec<u8> {
        let mut header = format!(
            "*{msg_size}\r\n\
             $6\r\nMSETNX\r\n",
            msg_size = (self.key_value_pairs.len() * 2) + 1,
        );

        for (key, value) in &self.key_value_pairs {
            header.push_str(&format!(
                "${key_length}\r\n{key}\r\n\
                 ${value_length}\r\n{value}\r\n",
                key = key,
                key_length = key.len(),
                value = value,
                value_length = value.len(),
            ));
        }

        header.into()
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

pub fn mset() -> SetMany {
    SetMany {
        key_value_pairs: Vec::new(),
    }
}

pub struct GetSet<T> {
    key: String,
    value: String,
    _t: PhantomData<T>,
}

impl<T> StructuredCommand for GetSet<T>
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

pub fn getset<T, K, V>(key: K, value: V) -> GetSet<T>
where
    K: Into<String>,
    V: Into<RedisArg>,
{
    GetSet {
        key: key.into(),
        value: value.into().0,
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
