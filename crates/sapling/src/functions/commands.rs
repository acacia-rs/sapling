//! `/function <name>`: answered with how many lines ran, or that there is no such function; a
//! function called from a running one answers nothing (research/captures/bds-functions.txt).

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, ParameterSpec, ParseError};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx};

use super::run::{self, Called};

pub(super) const NOT_FOUND: &str = "commands.function.functionNameNotFound";

/// A function's name, a word as BDS's file path parameter reads it.
pub(super) fn function_parameter(name: &str) -> ParameterSpec {
    ParameterSpec { name: name.into(), kind: ParamKind::FilePath, optional: false }
}

pub(crate) struct FunctionCommand {
    name: String,
}

impl CommandSyntax for FunctionCommand {
    fn spec() -> CommandSpec {
        let parameters = vec![function_parameter("name")];
        CommandSpec { name: "function".into(), aliases: Vec::new(), description: "Runs commands found in the corresponding function file.".into(), permission: Permission::Operator, overloads: vec![Overload { parameters }] }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        Ok(Self { name: args.word()?.to_owned() })
    }
}

impl Command for FunctionCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        match run::call(ctx, &self.name, sender) {
            Called::NotFound => out.translate(NOT_FOUND, vec![self.name], false),
            Called::Opened => {}
            Called::Ran(lines) => out.translate("commands.function.success", vec![lines.to_string()], true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_function_is_named_by_one_word() {
        let sender = Sender::Console;
        let parsed = |line: &str| {
            let mut args = Args::new(line, 0, &sender, &[]);
            FunctionCommand::parse(&mut args).and_then(|command| args.finish().map(|()| command.name)).ok()
        };
        assert_eq!([parsed("deep/a/b"), parsed("probe/hello extra"), parsed("")], [Some("deep/a/b".into()), None, None]);
    }
}
