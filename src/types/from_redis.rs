#[derive(Debug, PartialEq, Eq)]
pub enum RedisResponse {
    String(String),
    Integer(i64),
    Array(Vec<RedisResponse>),
    Error(String), // use a string for now, can be changed to a unique RedisError enum
    Null,
}

pub trait FromRedisValue: Sized {
    type Err;

    fn from_redis_value(r: &RedisResponse) -> Result<Self, Self::Err>;
}

impl FromRedisValue for () {
    type Err = String;

    fn from_redis_value(r: &RedisResponse) -> Result<Self, Self::Err> {
        match r {
            RedisResponse::String(_) => Ok(()),
            RedisResponse::Integer(_) => Ok(()),
            RedisResponse::Array(_) => Ok(()),
            RedisResponse::Error(text) => Err(text.to_string()),
            RedisResponse::Null => Ok(()),
        }
    }
}
