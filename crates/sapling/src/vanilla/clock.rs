//! `/time` and `/xp`, answering as BDS does (research/captures/bds-clock.txt).
// TODO: a named time is set on the day it is, as BDS did on day 0 and day 15; whether a time
// already past moves to the next day was not drilled.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx};

use super::{no_target, players};

const DAY: i64 = 24000;
const NAMED: [(&str, i64); 6] = [("day", 1000), ("noon", 6000), ("sunset", 12000), ("night", 13000), ("midnight", 18000), ("sunrise", 23000)];

pub(crate) enum Time {
    Set(i64),
    /// A time of the day it is.
    SetNamed(i64),
    Add(i64),
    Query(&'static str),
}

impl CommandSyntax for Time {
    fn spec() -> CommandSpec {
        let word = |name: &str| ParameterSpec { name: name.into(), kind: ParamKind::String, optional: false };
        CommandSpec { name: "time".into(), aliases: Vec::new(), description: "Changes or queries the world's game time".into(), permission: Permission::Operator, overloads: vec![Overload { parameters: vec![word("action"), word("time")] }] }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let action = if args.is_empty() { return Err(args.error_at(mark)) } else { args.word()? };
        let mark = args.mark();
        let value = if args.is_empty() { return Err(args.error_at(mark)) } else { args.word()? };
        let parsed = match (action, value.parse::<i64>()) {
            ("set", Ok(time)) => Some(Self::Set(time)),
            ("set", Err(_)) => NAMED.iter().find(|(name, _)| *name == value).map(|(_, time)| Self::SetNamed(*time)),
            ("add", Ok(by)) => Some(Self::Add(by)),
            ("query", _) => ["daytime", "gametime", "day"].into_iter().find(|what| *what == value).map(Self::Query),
            _ => None,
        };
        args.finish()?;
        parsed.ok_or_else(|| args.error_at(mark))
    }
}

impl Command for Time {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let now = ctx.time();
        match self {
            Self::Set(time) | Self::SetNamed(time) => {
                let time = if matches!(self, Self::SetNamed(_)) { now.div_euclid(DAY) * DAY + time } else { time };
                ctx.set_time(time);
                out.translate("commands.time.set", vec![time.to_string()], true);
            }
            Self::Add(by) => {
                ctx.set_time(now + by);
                out.translate("commands.time.added", vec![by.to_string()], true);
            }
            Self::Query(what) => {
                let answer = match what {
                    "daytime" => now.rem_euclid(DAY),
                    "day" => now.div_euclid(DAY),
                    _ => ctx.tick() as i64,
                };
                out.translate(&format!("commands.time.query.{what}"), vec![answer.to_string()], true);
            }
        }
    }
}

/// `/xp <amount>[L] [player]`: points, or levels with an `L`.
pub(crate) struct Xp {
    amount: i32,
    levels: bool,
    target: Option<Selector>,
}

impl CommandSyntax for Xp {
    fn spec() -> CommandSpec {
        let amount = ParameterSpec { name: "amount".into(), kind: ParamKind::String, optional: false };
        let player = ParameterSpec { name: "player".into(), kind: ParamKind::Target, optional: true };
        CommandSpec { name: "xp".into(), aliases: Vec::new(), description: "Adds or removes player experience".into(), permission: Permission::Operator, overloads: vec![Overload { parameters: vec![amount, player] }] }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let word = if args.is_empty() { return Err(args.error_at(mark)) } else { args.word()? };
        let (number, levels) = word.strip_suffix(['L', 'l']).map_or((word, false), |number| (number, true));
        let amount = number.parse::<i32>().map_err(|_| args.error_at(mark))?;
        let target = if args.is_empty() { None } else { Some(Selector::parse(args)?) };
        args.finish()?;
        Ok(Self { amount, levels, target })
    }
}

impl Command for Xp {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        if matches!(&self.target, Some(Selector::Entities { kind: Some(kind) }) if kind != "minecraft:player") {
            return out.translate("commands.generic.targetNotPlayer", Vec::new(), false);
        }
        if !self.levels && self.amount < 0 {
            return out.translate("commands.xp.failure.widthdrawXp", Vec::new(), false);
        }
        let found = players(ctx, sender, self.target.as_ref().unwrap_or(&Selector::Sender));
        if found.is_empty() {
            return no_target(out);
        }
        for player in found {
            let key = match (self.levels, self.amount < 0) {
                (false, _) => "commands.xp.success",
                (true, false) => "commands.xp.success.levels",
                (true, true) => "commands.xp.success.negative.levels",
            };
            match self.levels {
                true => ctx.add_levels(player.id, self.amount),
                false => ctx.give_experience(player.id, self.amount as u32),
            };
            out.translate(key, vec![self.amount.unsigned_abs().to_string(), player.name], true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_and_xp_take_what_bds_took() {
        let sender = Sender::Console;
        let time = |line: &str| Time::parse(&mut Args::new(line, 0, &sender, &[])).ok();
        assert!(matches!(time("set midnight"), Some(Time::SetNamed(18000))) && matches!(time("set -5"), Some(Time::Set(-5))) && matches!(time("add 500"), Some(Time::Add(500))));
        assert!(matches!(time("query day"), Some(Time::Query("day"))) && time("set banana").is_none() && time("").is_none() && time("query").is_none());
        let xp = |line: &str| Xp::parse(&mut Args::new(line, 0, &sender, &[])).ok().map(|xp| (xp.amount, xp.levels, xp.target.is_some()));
        assert_eq!([xp("7 @s"), xp("-1L @s"), xp("2l"), xp("banana @s")], [Some((7, false, true)), Some((-1, true, true)), Some((2, true, false)), None]);
    }
}
