use crate::types::redis_values::{ConversionError, RedisResult};
use crate::{RBytes, RedisValue};
use std::convert::TryInto;

pub trait StructuredCommand {
    type Output;

    fn get_bytes(&self) -> Vec<u8>;
    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError>;
}

#[derive(Debug, Eq, PartialEq)]
pub struct Command<'a> {
    pub(self) name: RBytes<'a>,
    pub(self) args: Vec<RBytes<'a>>,
}

impl<'a> Command<'a> {
    pub fn cmd(name: impl Into<RBytes<'a>>) -> Self {
        Command {
            name: name.into(),
            args: Vec::new(),
        }
    }

    pub fn cmd_with_args(
        name: impl Into<RBytes<'a>>,
        args: impl IntoIterator<Item = impl Into<RBytes<'a>>>,
    ) -> Self {
        Command {
            name: name.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    pub fn with_arg(mut self, argument: impl Into<RBytes<'a>>) -> Self {
        self.args.push(argument.into());
        self
    }
}

impl<'a> StructuredCommand for Command<'a> {
    type Output = Option<RedisValue>;

    fn get_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.push(b'*');
        result.extend_from_slice((self.args.len() + 1).to_string().as_bytes());
        result.extend_from_slice(b"\r\n");

        insert_bytes_into_vec!(result, &self.name);

        for arg in &(self.args) {
            insert_bytes_into_vec!(result, arg);
        }

        result
    }

    fn convert_redis_result(self, result: RedisResult) -> Result<Self::Output, ConversionError> {
        result.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_a_new_command_with_just_a_name() {
        let cmd = Command::cmd("MY_CMD");

        assert_eq!(cmd.name, RBytes::from("MY_CMD"));
        assert_eq!(cmd.args, Vec::<RBytes>::new());
    }

    #[test]
    fn can_create_a_new_command_with_name_and_args() {
        let cmd = Command::cmd_with_args("MY_CMD", vec!["1", "2"]);

        assert_eq!(cmd.name, RBytes::from("MY_CMD"));
        assert_eq!(cmd.args, vec![RBytes::from("1"), RBytes::from("2")]);
    }

    #[test]
    fn can_append_args_of_a_variety_of_types() {
        let cmd = Command::cmd("MY_CMD")
            .with_arg(1)
            .with_arg("test")
            .with_arg("String".to_string())
            .with_arg(false)
            .with_arg(3.4);

        assert_eq!(cmd.name, RBytes::from("MY_CMD"));
        assert_eq!(
            cmd.args,
            vec![
                RBytes::from("1"),
                RBytes::from("test"),
                RBytes::from("String"),
                RBytes::from("false"),
                RBytes::from("3.4")
            ]
        )
    }

    #[test]
    fn returns_the_correct_command_string_for_an_arbitrary_command() {
        let cmd = Command::cmd("MYCMD").with_arg(120).with_arg("test");

        assert_eq!(
            resp_bytes!("MYCMD", 120.to_string(), "test"),
            cmd.get_bytes()
        );
    }
}
