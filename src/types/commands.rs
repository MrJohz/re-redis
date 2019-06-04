use crate::types::command::RedisArg;
use crate::types::command::StructuredCommand;
use crate::utils::number_length;
use std::marker::PhantomData;
use std::time::Duration;

fn validate_key(key: impl Into<String>) -> String {
    let key = key.into();
    if key.len() > 512 * 1000 * 1000 {
        // 512 MB
        panic!("key is too large (over 512 MB)");
    }
    key
}

macro_rules! impl_stuctured_command {
    (
        $type_name:ident;
        $arg_name:ident => $as_bytes:block,
        $existing_type:ty
    ) => {
        impl StructuredCommand for $type_name {
            type Output = $existing_type;

            fn into_bytes(self) -> Vec<u8> {
                let $arg_name = self;
                $as_bytes
            }
        }
    };
    (
        $type_name:ident;
        $arg_name:ident => $as_bytes:block,
        $($existing_type:ty)|+
    ) => {
        $(
            impl StructuredCommand for $type_name<$existing_type> {
                type Output = $existing_type;

                fn into_bytes(self) -> Vec<u8> {
                    let $arg_name = self;
                    $as_bytes
                }
            }
        )*
    };
}

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

    fn into_bytes(self) -> Vec<u8> {
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
            )
            .into(),
            None => format!(
                "*3\r\n\
                 $3\r\nSET\r\n\
                 ${key_length}\r\n{key}\r\n\
                 ${value_length}\r\n{value}\r\n",
                key = self.key,
                key_length = self.key.len(),
                value = self.value,
                value_length = self.value.len(),
            )
            .into(),
        }
    }
}

impl_stuctured_command! {SetIfExists;
    this => {
        let exists_tag = if this.exists { "XX" } else { "NX" };
        match this.expiry {
            Some(duration) => format!("SET {} {} PX {} {}\r\n",
                this.key,
                this.value,
                duration.as_millis(),
                exists_tag
            ).into(),
            None => format!("SET {} {} {}\r\n",
                this.key,
                this.value,
                exists_tag
            ).into(),
        }
    },
    Option<()>
}

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

impl_stuctured_command! {Increment;
    this => {
        if this.by == 1 {
            format!("INCR {}\r\n", this.key).into()
        } else if this.by == -1 {
            format!("DECR {}\r\n", this.key).into()
        } else if this.by >= 0 {
            format!("INCRBY {} {}\r\n", this.key, this.by).into()
        } else {
            format!("DECRBY {} {}\r\n", this.key, -this.by).into()
        }
    },
    i64
}

pub struct Get<T> {
    key: String,
    _t: PhantomData<T>,
}

impl<T> Get<T> {
    pub(self) fn new(key: String) -> Self {
        Self {
            key,
            _t: PhantomData,
        }
    }
}

pub fn get<T>(key: impl Into<String>) -> Get<T> {
    Get::new(validate_key(key))
}

impl_stuctured_command! {Get;
    this => { format!(
        "*2\r\n\
        $3\r\nGET\r\n\
        ${}\r\n{}\r\n", this.key.len(), this.key).into() },
    Option<String> |
    Option<isize> | Option<i64> | Option<i32> | Option<i16> | Option<i8> |
    Option<usize> | Option<u64> | Option<u32> | Option<u16> | Option<u8> |
    Option<f64> | Option<f32>

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_command_converts_to_bytes_with_expiry_data() {
        let cmd = set("my-first-key", 42).with_expiry(Duration::from_secs(400));

        assert_eq!(
            String::from_utf8(cmd.into_bytes()).unwrap(),
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
            String::from_utf8(cmd.into_bytes()).unwrap(),
            "SET my-first-key 42 XX\r\n"
        );
    }

    #[test]
    fn set_if_exists_can_have_an_optional_duration() {
        let cmd = set("my-first-key", 42)
            .if_exists()
            .with_expiry(Duration::from_millis(1000));

        assert_eq!(
            String::from_utf8(cmd.into_bytes()).unwrap(),
            "SET my-first-key 42 PX 1000 XX\r\n"
        );
    }

    #[test]
    fn set_command_with_duration_keeps_duration_when_transformed_to_set_if_exists() {
        let cmd = set("my-first-key", 42)
            .with_expiry(Duration::from_millis(1000))
            .if_exists();

        assert_eq!(
            String::from_utf8(cmd.into_bytes()).unwrap(),
            "SET my-first-key 42 PX 1000 XX\r\n"
        );
    }

    #[test]
    fn set_command_can_transform_to_if_not_exists_format() {
        let cmd = set("my-first-key", 42).if_not_exists();

        assert_eq!(
            String::from_utf8(cmd.into_bytes()).unwrap(),
            "SET my-first-key 42 NX\r\n"
        );
    }

    #[test]
    fn incr_command_increments_by_one_by_default() {
        let cmd = incr("my-first-key");

        assert_eq!(
            String::from_utf8(cmd.into_bytes()).unwrap(),
            "INCR my-first-key\r\n"
        );
    }

    #[test]
    fn incr_by_command_increments_by_other_numbers_when_given() {
        let cmd = incr_by("my-first-key", 120);

        assert_eq!(
            String::from_utf8(cmd.into_bytes()).unwrap(),
            "INCRBY my-first-key 120\r\n"
        );
    }

    #[test]
    fn decr_by_command_decrements_by_given_value() {
        let cmd = decr_by("my-first-key", 120);

        assert_eq!(
            String::from_utf8(cmd.into_bytes()).unwrap(),
            "DECRBY my-first-key 120\r\n"
        );
    }

    #[test]
    fn decr_command_decrements_by_one() {
        let cmd = decr("my-first-key");

        assert_eq!(
            String::from_utf8(cmd.into_bytes()).unwrap(),
            "DECR my-first-key\r\n"
        );
    }
}
