//! `/execute in <dimension> run tp ...`, the one form of `/execute` sapling runs: the way to
//! another dimension without a portal, as BDS takes it (research/captures/bds-dims-travel.txt).
// TODO: the rest of `/execute` (as, at, positioned, if, unless, ...).

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, ParameterSpec, ParseError};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx};

use super::players::Teleport;

const DIMENSIONS: [(&str, i32); 3] = [("overworld", 0), ("nether", 1), ("the_end", 2)];

pub(crate) struct Execute {
    dimension: i32,
    teleport: Teleport,
}

impl CommandSyntax for Execute {
    fn spec() -> CommandSpec {
        let dimension = ParameterSpec { name: "dimension".into(), kind: ParamKind::Enum { name: "Dimension".into(), values: DIMENSIONS.map(|(name, _)| name.to_owned()).to_vec() }, optional: false };
        let command = ParameterSpec { name: "command".into(), kind: ParamKind::Text, optional: false };
        CommandSpec {
            name: "execute".into(),
            aliases: Vec::new(),
            description: "Executes a command on behalf of one or more entities".into(),
            permission: Permission::Operator,
            overloads: vec![Overload { parameters: vec![ParameterSpec::literal("execute", "in"), dimension, ParameterSpec::literal("execute", "run"), command] }],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let word = |args: &mut Args<'_>, wanted: &[&str]| -> Result<usize, ParseError> {
            let mark = args.mark();
            let word = args.word()?;
            wanted.iter().position(|one| word.eq_ignore_ascii_case(one)).ok_or_else(|| args.error_at(mark))
        };
        word(args, &["in"])?;
        let dimension = DIMENSIONS[word(args, &DIMENSIONS.map(|(name, _)| name))?].1;
        word(args, &["run"])?;
        word(args, &["tp", "teleport"])?;
        Ok(Self { dimension, teleport: Teleport::parse(args)? })
    }
}

impl Command for Execute {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        self.teleport.run_in(ctx, sender, out, Some(self.dimension));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_teleport_into_a_dimension_parses() {
        let sender = Sender::Console;
        let parsed = |line: &str| Execute::parse(&mut Args::new(line, 0, &sender, &[])).map(|execute| execute.dimension).ok();
        assert_eq!([parsed("in nether run tp @s 0 70 0"), parsed("in the_end run teleport 1 2 3"), parsed("in Overworld run tp ~ ~ ~")], [Some(1), Some(2), Some(0)]);
        assert_eq!([parsed("in hell run tp 0 0 0"), parsed("as @s run tp 0 0 0"), parsed("in nether run say hi")], [None, None, None]);
    }
}
