use crate::types::from_redis::FromRedisValue;
use crate::types::into_redis::IntoRedisValue;
use std::marker::PhantomData;

#[derive(Clone)]
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
        args: impl IntoIterator<Item = impl IntoRedisValue>,
    ) -> Self {
        Command {
            name: name.into(),
            args: args.into_iter().map(IntoRedisValue::convert).collect(),
        }
    }

    pub fn with_arg(&mut self, argument: impl IntoRedisValue) -> &mut Self {
        self.args.push(argument.convert());
        self
    }

    pub fn get_command_string(&self) -> String {
        let mut result = format!("{}", self.name);
        for arg in &(self.args) {
            result.push_str(" ");
            result.push_str(arg);
        }

        result
    }
}

pub trait StructuredCommand {
    type Output: FromRedisValue;

    fn get_command(&self) -> Command;
}

struct CommandSet {
    command: Command,
}

impl StructuredCommand for CommandSet {
    type Output = ();

    fn get_command(&self) -> Command {
        self.command.clone()
    }
}

struct CommandGet<T> {
    command: Command,
    _phantom_field: PhantomData<T>,
}

impl StructuredCommand for CommandGet<i64> {
    type Output = i64;

    fn get_command(&self) -> Command {
        self.command.clone()
    }
}

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
        let mut cmd = Command::cmd("MY_CMD");
        cmd.with_arg(1);
        cmd.with_arg("test");
        cmd.with_arg("String".to_string());
        cmd.with_arg(false);
        cmd.with_arg(3.4);
    }
}
