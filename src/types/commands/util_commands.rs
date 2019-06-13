use crate::types::redis_values::ConversionError;
use crate::{RedisResult, RedisValue, StructuredCommand};
use std::convert::TryInto;

pub struct Ping;

impl StructuredCommand for Ping {
    type Output = ();

    fn get_bytes(&self) -> Vec<u8> {
        resp_bytes!("PING")
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        match result {
            // TODO: test this in subscription mode
            RedisResult::String(string) => {
                if string == b"PONG" {
                    Ok(())
                } else {
                    Err(ConversionError::NoConversionTypeMatch {
                        value: Some(RedisValue::String(string)),
                    })
                }
            }
            RedisResult::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Err(ConversionError::NoConversionTypeMatch {
                value: result.try_into()?,
            }),
        }
    }
}

pub fn ping() -> Ping {
    Ping
}
