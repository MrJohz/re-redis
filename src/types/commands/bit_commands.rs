use crate::types::redis_values::ConversionError;
use crate::types::{RedisResult, StructuredCommand};
use crate::utils::{number_length, validate_key};
use std::convert::TryInto;
use std::ops::{Range, RangeFrom, RangeInclusive};

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

pub trait RangeWithBounds {
    fn into_bounds(self) -> (i64, i64);
}

impl RangeWithBounds for Range<i64> {
    fn into_bounds(self) -> (i64, i64) {
        (self.start, self.end)
    }
}

impl RangeWithBounds for RangeInclusive<i64> {
    fn into_bounds(self) -> (i64, i64) {
        let (start, end) = self.into_inner();
        (start, end - 1)
    }
}

pub trait RangeWithLowerBound {
    fn into_bounds(self) -> (i64, Option<i64>);
}

impl<T> RangeWithLowerBound for T
where
    T: RangeWithBounds,
{
    fn into_bounds(self) -> (i64, Option<i64>) {
        let (lower, upper) = self.into_bounds();
        (lower, Some(upper))
    }
}

impl RangeWithLowerBound for RangeFrom<i64> {
    fn into_bounds(self) -> (i64, Option<i64>) {
        (self.start, None)
    }
}

#[derive(Debug)]
pub struct BitCount {
    key: String,
    indices: Option<(i64, i64)>,
}

impl BitCount {
    pub fn in_range(mut self, range: impl RangeWithBounds) -> Self {
        self.indices.replace(range.into_bounds());
        self
    }
}

impl StructuredCommand for BitCount {
    type Output = u32;

    fn get_bytes(&self) -> Vec<u8> {
        match self.indices {
            Some((start, end)) => format!(
                "*4\r\n\
                 $8\r\nBITCOUNT\r\n\
                 ${key_length}\r\n{key}\r\n\
                 ${start_length}\r\n{start}\r\n\
                 ${end_length}\r\n{end}\r\n",
                key_length = self.key.len(),
                key = self.key,
                start_length = number_length(start as i128),
                start = start,
                end_length = number_length(end as i128),
                end = end
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
            RedisResult::Integer(number @ 0...std::i64::MAX) => Ok(number as u32),
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

pub struct BitPos {
    key: String,
    bit: bool,
    // if a bound is given, there *must* be a lower bound, and there *may* be an upper bound
    range: Option<(i64, Option<i64>)>,
}

impl BitPos {
    pub fn in_range(mut self, range: impl RangeWithLowerBound) -> Self {
        self.range.replace(range.into_bounds());
        self
    }
}

impl StructuredCommand for BitPos {
    type Output = Option<u32>;

    fn get_bytes(&self) -> Vec<u8> {
        match self.range {
            Some((start, Some(end))) => format!(
                "*5\r\n\
                 $6\r\nBITPOS\r\n\
                 ${key_len}\r\n{key}\r\n\
                 $1\r\n{bit}\r\n\
                 ${start_length}\r\n{start}\r\n\
                 ${end_length}\r\n{end}\r\n",
                key_len = self.key.len(),
                key = self.key,
                bit = if self.bit { "1" } else { "0" },
                start_length = number_length(start as i128),
                start = start,
                end_length = number_length(end as i128),
                end = end,
            ),
            Some((start, None)) => format!(
                "*4\r\n\
                 $6\r\nBITPOS\r\n\
                 ${key_len}\r\n{key}\r\n\
                 $1\r\n{bit}\r\n\
                 ${start_length}\r\n{start}\r\n",
                key_len = self.key.len(),
                key = self.key,
                bit = if self.bit { "1" } else { "0" },
                start_length = number_length(start as i128),
                start = start,
            ),
            None => format!(
                "*3\r\n\
                 $6\r\nBITPOS\r\n\
                 ${key_len}\r\n{key}\r\n\
                 $1\r\n{bit}\r\n",
                key_len = self.key.len(),
                key = self.key,
                bit = if self.bit { "1" } else { "0" },
            ),
        }
        .into()
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        match result {
            RedisResult::Integer(number @ 0...std::i64::MAX) => Ok(Some(number as u32)),
            RedisResult::Integer(-1) => Ok(None),
            RedisResult::Error(error) => Err(ConversionError::RedisReturnedError { error }),
            _ => Err(ConversionError::NoConversionTypeMatch {
                value: result.try_into()?,
            }),
        }
    }
}

pub fn bitpos(key: impl Into<String>, bit: bool) -> BitPos {
    BitPos {
        key: key.into(),
        bit,
        range: None,
    }
}
