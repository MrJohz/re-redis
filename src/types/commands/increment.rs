use std::convert::TryInto;

use crate::types::redis_values::{ConversionError, RedisResult};
use crate::types::StructuredCommand;
use crate::utils::validate_key;

// TODO: Convert to RBytes
pub struct Increment {
    key: String,
    by: i64,
}

impl Increment {
    pub(self) fn new(key: String, by: i64) -> Self {
        Self { key, by }
    }
}

pub fn incr(key: impl Into<String>) -> Increment {
    Increment::new(validate_key(key), 1)
}
pub fn incr_by(key: impl Into<String>, by: i64) -> Increment {
    Increment::new(validate_key(key), by)
}
pub fn decr(key: impl Into<String>) -> Increment {
    Increment::new(validate_key(key), -1)
}
pub fn decr_by(key: impl Into<String>, by: i64) -> Increment {
    Increment::new(validate_key(key), -by)
}

impl StructuredCommand for Increment {
    type Output = i64;

    fn get_bytes(&self) -> Vec<u8> {
        if self.by == 1 {
            format!("INCR {}\r\n", self.key).into()
        } else if self.by == -1 {
            format!("DECR {}\r\n", self.key).into()
        } else if self.by >= 0 {
            format!("INCRBY {} {}\r\n", self.key, self.by).into()
        } else {
            format!("DECRBY {} {}\r\n", self.key, -self.by).into()
        }
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        result.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incr_command_increments_by_one_by_default() {
        let cmd = incr("my-first-key");

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "INCR my-first-key\r\n"
        );
    }

    #[test]
    fn incr_by_command_increments_by_other_numbers_when_given() {
        let cmd = incr_by("my-first-key", 120);

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "INCRBY my-first-key 120\r\n"
        );
    }

    #[test]
    fn decr_by_command_decrements_by_given_value() {
        let cmd = decr_by("my-first-key", 120);

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "DECRBY my-first-key 120\r\n"
        );
    }

    #[test]
    fn decr_command_decrements_by_one() {
        let cmd = decr("my-first-key");

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "DECR my-first-key\r\n"
        );
    }
}
