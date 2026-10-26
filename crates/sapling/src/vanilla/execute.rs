//! `/execute in <dimension> run tp ...`, the way to another dimension without a portal, as BDS takes
//! it (research/captures/bds-dims-travel.txt), and `/execute at <target> run tp ...`, a teleport
//! from where each entity named stands (bds-dolphin.txt, bds-pandas.txt). The only forms sapling runs.
// TODO: the rest of `/execute` (as, positioned, if, unless, ...) and commands other than `tp`.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx};

use super::teleport::Teleport;
use super::{no_target, targets};

const DIMENSIONS: [(&str, i32); 3] = [("overworld", 0), ("nether", 1), ("the_end", 2)];

pub(crate) struct Execute {
    context: Context,
    teleport: Teleport,
}

enum Context {
    In(i32),
    At(Selector),
}

impl CommandSyntax for Execute {
    fn spec() -> CommandSpec {
        let dimension = ParameterSpec { name: "dimension".into(), kind: ParamKind::Enum { name: "Dimension".into(), values: DIMENSIONS.map(|(name, _)| name.to_owned()).to_vec() }, optional: false };
        let command = || ParameterSpec { name: "command".into(), kind: ParamKind::Text, optional: false };
        let runs = |first: Vec<ParameterSpec>| Overload { parameters: first.into_iter().chain([ParameterSpec::literal("execute", "run"), command()]).collect() };
        CommandSpec {
            name: "execute".into(),
            aliases: Vec::new(),
            description: "Executes a command on behalf of one or more entities".into(),
            permission: Permission::Operator,
            overloads: vec![runs(vec![ParameterSpec::literal("execute", "in"), dimension]), runs(vec![ParameterSpec::literal("execute", "at"), ParameterSpec::of::<Selector>("origin")])],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let word = |args: &mut Args<'_>, wanted: &[&str]| -> Result<usize, ParseError> {
            let mark = args.mark();
            let word = args.word()?;
            wanted.iter().position(|one| word.eq_ignore_ascii_case(one)).ok_or_else(|| args.error_at(mark))
        };
        let context = match word(args, &["in", "at"])? {
            0 => Context::In(DIMENSIONS[word(args, &DIMENSIONS.map(|(name, _)| name))?].1),
            _ => Context::At(Selector::parse(args)?),
        };
        word(args, &["run"])?;
        word(args, &["tp", "teleport"])?;
        Ok(Self { context, teleport: Teleport::parse(args)? })
    }
}

impl Command for Execute {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        match self.context {
            Context::In(dimension) => {
                let here = ctx.origin(sender);
                self.teleport.run_from(ctx, sender, out, Some(dimension), here);
            }
            Context::At(selector) => {
                let (people, others) = targets(ctx, sender, Some(&selector));
                let stood: Vec<[f32; 3]> = people.into_iter().filter_map(|player| ctx.position(player.id).map(|at| at.feet)).chain(others.into_iter().map(|entity| entity.feet)).collect();
                if stood.is_empty() {
                    return no_target(out);
                }
                for here in stood {
                    self.teleport.clone().run_from(ctx, sender, out, None, here);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_teleport_into_a_dimension_or_from_an_entity_parses() {
        let sender = Sender::Console;
        let parsed = |line: &str| Execute::parse(&mut Args::new(line, 0, &sender, &[])).map(|execute| matches!(execute.context, Context::In(_))).ok();
        assert_eq!([parsed("in nether run tp @s 0 70 0"), parsed("in Overworld run tp ~ ~ ~"), parsed("at @e[type=panda] run tp @s ~ ~ ~-1.5 facing ~ ~1 ~")], [Some(true), Some(true), Some(false)]);
        assert_eq!([parsed("in hell run tp 0 0 0"), parsed("as @s run tp 0 0 0"), parsed("in nether run say hi")], [None, None, None]);
    }
}
