//! `/gamerule`: lists, shows or sets the sender's world's game rules, answering as BDS does
//! (research/captures/bds-gamerule.txt).
// TODO: BDS completes the rule names (one overload per kind); here both arguments are plain words.

use sapling_api::command::{
    Args, CommandSpec, CommandSyntax, Overload, ParamKind, ParameterSpec, ParseError,
};
use sapling_api::{Command, Output, Permission, RuleValue, Sender, ShardCtx};

/// BDS's bounds on the numbers it bounds: rule, least, most.
const BOUNDS: &[(&str, Option<i32>, Option<i32>)] = &[
    ("randomTickSpeed", Some(0), Some(4096)),
    ("spawnRadius", Some(0), Some(128)),
    ("functionCommandLimit", None, Some(10_000)),
];
/// A number set and shown as a word.
const WAYPOINTS: &str = "playerWaypoints";
const WAYPOINT_WORDS: &[(&str, i32)] = &[("off", 0), ("everyone", 1)];
/// Listed, but BDS's parser does not know it.
const UNSETTABLE: &str = "locatorbar";

/// An argument as typed, and how BDS reports it when it is wrong.
struct Word {
    text: String,
    wrong: ParseError,
}

fn word(args: &mut Args<'_>) -> Result<Option<Word>, ParseError> {
    if args.is_empty() {
        return Ok(None);
    }
    let mark = args.mark();
    let text = args.word()?.to_owned();
    Ok(Some(Word {
        text,
        wrong: args.error_at(mark),
    }))
}

pub(crate) struct GameRule {
    rule: Option<Word>,
    value: Option<Word>,
    /// How BDS reports a value left out where one is needed.
    missing: ParseError,
}

impl CommandSyntax for GameRule {
    fn spec() -> CommandSpec {
        let optional = |name: &str| ParameterSpec {
            name: name.into(),
            kind: ParamKind::String,
            optional: true,
        };
        CommandSpec {
            name: "gamerule".into(),
            aliases: Vec::new(),
            description: "Sets or queries a game rule value".into(),
            permission: Permission::Operator,
            overloads: vec![Overload {
                parameters: vec![optional("rule"), optional("value")],
            }],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let (rule, value) = (word(args)?, word(args)?);
        args.finish()?;
        Ok(Self {
            rule,
            value,
            missing: args.error_at(args.mark()),
        })
    }
}

/// A value as `/gamerule` shows it.
fn shown(rule: &str, value: RuleValue) -> String {
    let word = WAYPOINT_WORDS
        .iter()
        .find(|(_, n)| rule == WAYPOINTS && value == RuleValue::Int(*n));
    word.map_or_else(|| value.to_string(), |(word, _)| (*word).to_owned())
}

/// Why a typed value was not taken.
enum Refused {
    Wrong,
    TooSmall(i32),
    TooBig(i32),
}

/// What `text` sets a rule holding `now` to.
fn value_for(rule: &str, now: RuleValue, text: &str) -> Result<RuleValue, Refused> {
    let value = match now {
        _ if rule == WAYPOINTS => WAYPOINT_WORDS
            .iter()
            .find(|(word, _)| word.eq_ignore_ascii_case(text))
            .map(|(_, n)| RuleValue::Int(*n))
            .ok_or(Refused::Wrong)?,
        RuleValue::Bool(_) => RuleValue::Bool(
            text.to_ascii_lowercase()
                .parse()
                .map_err(|_| Refused::Wrong)?,
        ),
        RuleValue::Int(_) => RuleValue::Int(text.parse().map_err(|_| Refused::Wrong)?),
    };
    let (least, most) = BOUNDS
        .iter()
        .find(|(name, ..)| *name == rule)
        .map_or((None, None), |(_, least, most)| (*least, *most));
    match value {
        RuleValue::Int(n) => match (
            least.filter(|least| n < *least),
            most.filter(|most| n > *most),
        ) {
            (Some(least), _) => Err(Refused::TooSmall(least)),
            (_, Some(most)) => Err(Refused::TooBig(most)),
            _ => Ok(value),
        },
        RuleValue::Bool(_) => Ok(value),
    }
}

impl Command for GameRule {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let Some(typed) = self.rule else {
            let rules: Vec<String> = ctx
                .game_rules()
                .into_iter()
                .map(|(name, value)| format!("{name} = {}", shown(&name, value)))
                .collect();
            return out.print(rules.join(", "));
        };
        let Some((rule, now)) = ctx
            .game_rule(&typed.text)
            .filter(|(rule, _)| !rule.eq_ignore_ascii_case(UNSETTABLE))
        else {
            return out.refuse(typed.wrong);
        };
        let Some(value) = self.value else {
            return match rule == WAYPOINTS {
                true => out.refuse(self.missing),
                false => out.print(format!("{} = {}", typed.text, shown(&rule, now))),
            };
        };
        match value_for(&rule, now, &value.text) {
            Ok(new) if ctx.set_game_rule(&rule, new) => out.translate(
                "commands.gamerule.success",
                vec![typed.text, value.text],
                true,
            ),
            Ok(_) => out.refuse(typed.wrong),
            Err(Refused::Wrong) => out.refuse(value.wrong),
            Err(Refused::TooSmall(least)) => out.translate(
                "commands.generic.num.tooSmall",
                vec![value.text, least.to_string()],
                false,
            ),
            Err(Refused::TooBig(most)) => out.translate(
                "commands.generic.num.tooBig",
                vec![value.text, most.to_string()],
                false,
            ),
        }
    }
}
