use crate::types::redis_values::ConversionError;
use crate::types::{RedisResult, StructuredCommand};
use crate::RBytes;
use std::convert::TryInto;
use std::ops::{Range, RangeFrom, RangeInclusive};

#[derive(Debug)]
pub struct SetBit<'a> {
    key: RBytes<'a>,
    offset: u32,
    value: bool,
}

impl<'a> StructuredCommand for SetBit<'a> {
    type Output = bool;

    fn get_bytes(&self) -> Vec<u8> {
        resp_bytes!(
            "SETBIT",
            self.key,
            self.offset.to_string(),
            if self.value { "1" } else { "0" }
        )
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

pub fn setbit<'a>(key: impl Into<RBytes<'a>>, offset: u32, value: bool) -> SetBit<'a> {
    SetBit {
        key: key.into(),
        offset,
        value,
    }
}

#[derive(Debug)]
pub struct GetBit<'a> {
    key: RBytes<'a>,
    offset: u32,
}

impl<'a> StructuredCommand for GetBit<'a> {
    type Output = bool;

    fn get_bytes(&self) -> Vec<u8> {
        resp_bytes!("GETBIT", self.key, self.offset.to_string())
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

pub fn getbit<'a>(key: impl Into<RBytes<'a>>, offset: u32) -> GetBit<'a> {
    GetBit {
        key: key.into(),
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
pub struct BitCount<'a> {
    key: RBytes<'a>,
    indices: Option<(i64, i64)>,
}

impl<'a> BitCount<'a> {
    pub fn in_range(mut self, range: impl RangeWithBounds) -> Self {
        self.indices.replace(range.into_bounds());
        self
    }
}

impl<'a> StructuredCommand for BitCount<'a> {
    type Output = u32;

    fn get_bytes(&self) -> Vec<u8> {
        match self.indices {
            Some((start, end)) => {
                resp_bytes!("BITCOUNT", self.key, start.to_string(), end.to_string())
            }
            None => resp_bytes!("BITCOUNT", self.key),
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

pub fn bitcount<'a>(key: impl Into<RBytes<'a>>) -> BitCount<'a> {
    BitCount {
        key: key.into(),
        indices: None,
    }
}

pub struct BitPos<'a> {
    key: RBytes<'a>,
    bit: bool,
    // if a bound is given, there *must* be a lower bound, and there *may* be an upper bound
    range: Option<(i64, Option<i64>)>,
}

impl<'a> BitPos<'a> {
    pub fn in_range(mut self, range: impl RangeWithLowerBound) -> Self {
        self.range.replace(range.into_bounds());
        self
    }
}

impl<'a> StructuredCommand for BitPos<'a> {
    type Output = Option<u32>;

    fn get_bytes(&self) -> Vec<u8> {
        match self.range {
            Some((start, Some(end))) => resp_bytes!(
                "BITPOS",
                self.key,
                if self.bit { "1" } else { "0" },
                start.to_string(),
                end.to_string()
            ),
            Some((start, None)) => resp_bytes!(
                "BITPOS",
                self.key,
                if self.bit { "1" } else { "0" },
                start.to_string()
            ),
            None => resp_bytes!("BITPOS", self.key, if self.bit { "1" } else { "0" }),
        }
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

pub fn bitpos<'a>(key: impl Into<RBytes<'a>>, bit: bool) -> BitPos<'a> {
    BitPos {
        key: key.into(),
        bit,
        range: None,
    }
}
