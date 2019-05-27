use crate::types::redis_values::RedisResult;
use crate::RedisValue;
use std::convert::TryFrom;

pub trait StructuredCommand {
    type Output: TryFrom<RedisResult>;

    fn into_bytes(self) -> Vec<u8>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Command {
    pub(self) name: String,
    pub(self) args: Vec<String>,
}

impl Command {
    pub fn cmd(name: impl Into<String>) -> Self {
        Command {
            name: name.into(),
            args: Vec::new(),
        }
    }

    pub fn cmd_with_args(
        name: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<RedisArg>>,
    ) -> Self {
        Command {
            name: name.into(),
            args: args.into_iter().map(Into::into).map(|arg| arg.0).collect(),
        }
    }

    pub fn with_arg(mut self, argument: impl Into<RedisArg>) -> Self {
        self.args.push(argument.into().0);
        self
    }
}

impl StructuredCommand for Command {
    type Output = Option<RedisValue>;

    fn into_bytes(self) -> Vec<u8> {
        let mut result = self.name;
        for arg in &(self.args) {
            result.push_str(" ");
            result.push_str(arg);
        }

        result.push_str("\r\n");
        result.into()
    }
}

pub mod commands {
    use super::RedisArg;
    use super::StructuredCommand;
    use std::marker::PhantomData;

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
        value: String,
    }

    impl Set {
        pub(self) fn new(key: impl Into<String>, value: impl Into<RedisArg>) -> Self {
            Self {
                key: key.into(),
                value: value.into().0,
            }
        }
    }

    pub fn set(key: impl Into<String>, value: impl Into<RedisArg>) -> Set {
        Set::new(key, value)
    }

    impl_stuctured_command! {Set;
        this => { format!("SET {} {}\r\n", this.key, this.value).into() },
        ()
    }

    pub struct Get<T> {
        key: String,
        _t: PhantomData<T>,
    }

    impl<T> Get<T> {
        pub(self) fn new(key: impl Into<String>) -> Self {
            Self {
                key: key.into(),
                _t: PhantomData,
            }
        }
    }

    pub fn get<T>(key: impl Into<String>) -> Get<T> {
        Get::new(key)
    }

    impl_stuctured_command! {Get;
        this => { format!("GET {}\r\n", this.key).into() },
        Option<String> |
        Option<isize> | Option<i64> | Option<i32> | Option<i16> | Option<i8> |
        Option<usize> | Option<u64> | Option<u32> | Option<u16> | Option<u8> |
        Option<f64> | Option<f32>

    }
}

pub struct RedisArg(String);

macro_rules! create_convert_to_redis_arg_impl {
    ($kind:ty, $name:ident => $conversion:block) => {
        impl From<$kind> for RedisArg {
            fn from($name: $kind) -> Self {
                RedisArg($conversion)
            }
        }
    };
    ($kind:ty, $name:ident => $conversion:expr) => {
        impl From<$kind> for RedisArg {
            fn from($name: $kind) -> Self {
                RedisArg($conversion)
            }
        }
    };
    ($kind:ty, default) => {
        impl From<$kind> for RedisArg {
            fn from(element: $kind) -> Self {
                RedisArg(element.to_string())
            }
        }
    };
}

create_convert_to_redis_arg_impl! {isize, default}
create_convert_to_redis_arg_impl! {i64, default}
create_convert_to_redis_arg_impl! {i32, default}
create_convert_to_redis_arg_impl! {i16, default}
create_convert_to_redis_arg_impl! {i8, default}

create_convert_to_redis_arg_impl! {usize, default}
create_convert_to_redis_arg_impl! {u64, default}
create_convert_to_redis_arg_impl! {u32, default}
create_convert_to_redis_arg_impl! {u16, default}
create_convert_to_redis_arg_impl! {u8, default}

create_convert_to_redis_arg_impl! {f64, default}
create_convert_to_redis_arg_impl! {f32, default}

create_convert_to_redis_arg_impl! {bool, default}
create_convert_to_redis_arg_impl! {char, default}

create_convert_to_redis_arg_impl! {&str, default}
create_convert_to_redis_arg_impl! {String, input => input}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_a_new_command_with_just_a_name() {
        let cmd = Command::cmd("MY_CMD");

        assert_eq!(cmd.name, "MY_CMD".to_string());
        assert_eq!(cmd.args, Vec::<String>::new());
    }

    #[test]
    fn can_create_a_new_command_with_name_and_args() {
        let cmd = Command::cmd_with_args("MY_CMD", vec!["1", "2"]);

        assert_eq!(cmd.name, "MY_CMD".to_string());
        assert_eq!(cmd.args, vec!["1".to_string(), "2".to_string()]);
    }

    #[test]
    fn can_append_args_of_a_variety_of_types() {
        let cmd = Command::cmd("MY_CMD")
            .with_arg(1)
            .with_arg("test")
            .with_arg("String".to_string())
            .with_arg(false)
            .with_arg(3.4);

        assert_eq!(cmd.name, "MY_CMD".to_string());
        assert_eq!(
            cmd.args,
            vec![
                "1".to_string(),
                "test".to_string(),
                "String".to_string(),
                "false".to_string(),
                "3.4".to_string()
            ]
        )
    }
}
