use crate::types::redis_values::ConversionError;
use crate::types::{RedisResult, StructuredCommand};
use crate::utils::{number_length, validate_key};
use std::convert::TryInto;
use std::ops::Range;

#[derive(Debug)]
pub struct SetBit {
    key: String,
    offset: u32,
    value: bool,
}

impl StructuredCommand for SetBit {
    type Output = bool;

    fn get_bytes(&self) -> Vec<u8> {
        format!(
            "*4\r\n\
             $6\r\nSETBIT\r\n\
             ${key_length}\r\n{key}\r\n\
             ${offset_length}\r\n{offset}\r\n\
             $1\r\n{value}\r\n",
            key_length = self.key.len(),
            key = self.key,
            offset_length = number_length(self.offset as i128),
            offset = self.offset,
            value = if self.value { "1" } else { "0" }
        )
        .into()
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        match result {
            RedisResult::Integer(1) => Ok(true),
            RedisResult::Integer(0) => Ok(false),
            RedisResult::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Err(ConversionError::NoConversionTypeMatch {
                value: result.try_into()?,
            }),
        }
    }
}

pub fn setbit(key: impl Into<String>, offset: u32, value: bool) -> SetBit {
    SetBit {
        key: validate_key(key),
        offset,
        value,
    }
}

#[derive(Debug)]
pub struct GetBit {
    key: String,
    offset: u32,
}

impl StructuredCommand for GetBit {
    type Output = bool;

    fn get_bytes(&self) -> Vec<u8> {
        format!(
            "*3\r\n\
             $6\r\nGETBIT\r\n\
             ${key_length}\r\n{key}\r\n\
             ${offset_length}\r\n{offset}\r\n",
            key_length = self.key.len(),
            key = self.key,
            offset_length = number_length(self.offset as i128),
            offset = self.offset,
        )
        .into()
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        match result {
            RedisResult::Integer(1) => Ok(true),
            RedisResult::Integer(0) => Ok(false),
            RedisResult::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Err(ConversionError::NoConversionTypeMatch {
                value: result.try_into()?,
            }),
        }
    }
}

pub fn getbit(key: impl Into<String>, offset: u32) -> GetBit {
    GetBit {
        key: validate_key(key),
        offset,
    }
}

#[derive(Debug)]
pub struct BitCount {
    key: String,
    indices: Option<Range<i32>>,
}

impl BitCount {
    pub fn in_range(mut self, range: Range<i32>) -> BitCount {
        self.indices.replace(range);
        self
    }
}

impl StructuredCommand for BitCount {
    type Output = u32;

    fn get_bytes(&self) -> Vec<u8> {
        match &self.indices {
            Some(range) => format!(
                "*4\r\n\
                 $8\r\nBITCOUNT\r\n\
                 ${key_length}\r\n{key}\r\n\
                 ${start_length}\r\n{start}\r\n\
                 ${end_length}\r\n{end}\r\n",
                key_length = self.key.len(),
                key = self.key,
                start_length = number_length(range.start as i128),
                start = range.start,
                end_length = number_length(range.end as i128),
                end = range.end
            ),
            None => format!(
                "*2\r\n\
                 $8\r\nBITCOUNT\r\n\
                 ${key_length}\r\n{key}\r\n",
                key_length = self.key.len(),
                key = self.key
            ),
        }
        .into()
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        match result {
            RedisResult::Integer(number) => Ok(number as u32),
            RedisResult::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Err(ConversionError::NoConversionTypeMatch {
                value: result.try_into()?,
            }),
        }
    }
}

pub fn bitcount(key: impl Into<String>) -> BitCount {
    BitCount {
        key: validate_key(key),
        indices: None,
    }
}
