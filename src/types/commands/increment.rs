use std::convert::TryInto;

use crate::types::redis_values::{ConversionError, RedisResult};
use crate::types::StructuredCommand;
use crate::RBytes;

pub struct Increment<'a> {
    key: RBytes<'a>,
    by: i64,
}

impl<'a> Increment<'a> {
    pub(self) fn new(key: RBytes<'a>, by: i64) -> Self {
        Self { key, by }
    }
}

pub fn incr<'a>(key: impl Into<RBytes<'a>>) -> Increment<'a> {
    Increment::new(key.into(), 1)
}
pub fn incr_by<'a>(key: impl Into<RBytes<'a>>, by: i64) -> Increment<'a> {
    Increment::new(key.into(), by)
}
pub fn decr<'a>(key: impl Into<RBytes<'a>>) -> Increment<'a> {
    Increment::new(key.into(), -1)
}
pub fn decr_by<'a>(key: impl Into<RBytes<'a>>, by: i64) -> Increment<'a> {
    Increment::new(key.into(), -by)
}

impl<'a> StructuredCommand for Increment<'a> {
    type Output = i64;

    fn get_bytes(&self) -> Vec<u8> {
        if self.by == 1 {
            resp_bytes!("INCR", &self.key)
        } else if self.by == -1 {
            resp_bytes!("DECR", &self.key)
        } else if self.by >= 0 {
            resp_bytes!("INCRBY", &self.key, &self.by.to_string())
        } else {
            resp_bytes!("DECRBY", &self.key, &(-self.by).to_string())
        }
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        result.try_into()
    }
}

pub struct FloatIncrement<'a> {
    key: RBytes<'a>,
    by: f64,
}

pub fn incr_by_float<'a>(key: impl Into<RBytes<'a>>, by: f64) -> FloatIncrement<'a> {
    FloatIncrement {
        key: key.into(),
        by,
    }
}

pub fn decr_by_float<'a>(key: impl Into<RBytes<'a>>, by: f64) -> FloatIncrement<'a> {
    FloatIncrement {
        key: key.into(),
        by: -by,
    }
}

impl<'a> StructuredCommand for FloatIncrement<'a> {
    type Output = f64;

    fn get_bytes(&self) -> Vec<u8> {
        resp_bytes!("INCRBYFLOAT", &self.key, &self.by.to_string())
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
            "*2\r\n$4\r\nINCR\r\n$12\r\nmy-first-key\r\n"
        );
    }

    #[test]
    fn incr_by_command_increments_by_other_numbers_when_given() {
        let cmd = incr_by("my-first-key", 120);

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "*3\r\n$6\r\nINCRBY\r\n$12\r\nmy-first-key\r\n$3\r\n120\r\n"
        );
    }

    #[test]
    fn decr_by_command_decrements_by_given_value() {
        let cmd = decr_by("my-first-key", 120);

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "*3\r\n$6\r\nDECRBY\r\n$12\r\nmy-first-key\r\n$3\r\n120\r\n"
        );
    }

    #[test]
    fn decr_command_decrements_by_one() {
        let cmd = decr("my-first-key");

        assert_eq!(
            String::from_utf8(cmd.get_bytes()).unwrap(),
            "*2\r\n$4\r\nDECR\r\n$12\r\nmy-first-key\r\n"
        );
    }
}
