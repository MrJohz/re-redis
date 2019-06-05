use std::convert::TryInto;
use std::time::Duration;

use crate::types::command::RedisArg;
use crate::types::redis_values::{ConversionError, RedisResult};
use crate::types::StructuredCommand;
use crate::utils::{number_length, validate_key};

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
            Some(duration) => format!(
                "*5\r\n\
                 $3\r\nSET\r\n\
                 ${key_length}\r\n{key}\r\n\
                 ${value_length}\r\n{value}\r\n\
                 $2\r\nPX\r\n\
                 ${expiry_length}\r\n{expiry}\r\n",
                key = self.key,
                key_length = self.key.len(),
                value = self.value,
                value_length = self.value.len(),
                expiry_length = number_length(duration.as_millis()),
                expiry = duration.as_millis(),
            ),
            None => format!(
                "*3\r\n\
                 $3\r\nSET\r\n\
                 ${key_length}\r\n{key}\r\n\
                 ${value_length}\r\n{value}\r\n",
                key = self.key,
                key_length = self.key.len(),
                value = self.value,
                value_length = self.value.len(),
            ),
        }
        .into()
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
            Some(duration) => format!(
                "*6\r\n\
                 $3\r\nSET\r\n\
                 ${key_length}\r\n{key}\r\n\
                 ${value_length}\r\n{value}\r\n\
                 $2\r\nPX\r\n\
                 ${expiry_length}\r\n{expiry}\r\n\
                 $2\r\n{exists_tag}\r\n",
                key = self.key,
                key_length = self.key.len(),
                value = self.value,
                value_length = self.value.len(),
                expiry_length = number_length(duration.as_millis()),
                expiry = duration.as_millis(),
                exists_tag = exists_tag,
            ),
            None => format!(
                "*4\r\n\
                 $3\r\nSET\r\n\
                 ${key_length}\r\n{key}\r\n\
                 ${value_length}\r\n{value}\r\n\
                 $2\r\n{exists_tag}\r\n",
                key = self.key,
                key_length = self.key.len(),
                value = self.value,
                value_length = self.value.len(),
                exists_tag = exists_tag,
            ),
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
