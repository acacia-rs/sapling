//! `/schedule delay add|clear` and what runs every tick, as BDS answered and ran them
//! (research/captures/bds-functions.txt): a function is run by the server, in the world it was
//! scheduled in, once its delay (`20`, `2s`, `1d`) has passed, and the operators are told; a delay
//! of 0 runs it at once. `on_area_loaded` waits for ticking areas.
// TODO: `/schedule on_area_loaded`, once ticking areas exist.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, ParameterSpec, ParseError, RawPart, RawText};
use sapling_api::{Command, Output, Permission, RuleValue, Sender, ShardCtx};

use super::commands::{NOT_FOUND, function_parameter};
use super::run::{self, Called, FunctionState};

const TICKS_PER_SECOND: i64 = 20;
const TICKS_PER_DAY: i64 = 24_000;

struct Pending {
    due: u64,
    function: String,
    world: String,
}

/// The functions waiting for their delay on this shard, in the order they were scheduled.
#[derive(Default)]
struct Scheduled(Vec<Pending>);

pub(crate) enum Schedule {
    Add { function: String, ticks: i64, append: bool },
    Clear { function: String },
}

/// `20`, `20t`, `2s` or `1d`, in ticks.
fn ticks(word: &str) -> Option<i64> {
    let (number, scale) = match word.char_indices().last()? {
        (at, 't' | 'T') => (&word[..at], 1),
        (at, 's' | 'S') => (&word[..at], TICKS_PER_SECOND),
        (at, 'd' | 'D') => (&word[..at], TICKS_PER_DAY),
        _ => (word, 1),
    };
    number.parse::<i64>().ok()?.checked_mul(scale)
}

impl CommandSyntax for Schedule {
    fn spec() -> CommandSpec {
        let word = |command: &str, word: &str| ParameterSpec::literal(command, word);
        // BDS's time is an int or a suffixed int; a string takes both.
        let time = ParameterSpec { name: "time".into(), kind: ParamKind::String, optional: false };
        let mode = ParameterSpec { name: "DelayMode".into(), kind: ParamKind::Enum { name: "DelayMode".into(), values: vec!["replace".into(), "append".into()] }, optional: true };
        let add = vec![word("schedule", "delay"), word("schedule", "add"), function_parameter("function"), time, mode];
        let clear = vec![word("schedule", "delay"), word("schedule", "clear"), function_parameter("function")];
        let clear_all = vec![word("schedule", "clear"), function_parameter("function")];
        let overloads = [add, clear, clear_all].into_iter().map(|parameters| Overload { parameters }).collect();
        CommandSpec { name: "schedule".into(), aliases: Vec::new(), description: "Schedules an action to be executed once an area is loaded, or after a certain amount of time.".into(), permission: Permission::Operator, overloads }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let keyword = |args: &mut Args<'_>, wanted: &[&str]| -> Result<usize, ParseError> {
            let mark = args.mark();
            let word = args.word()?;
            wanted.iter().position(|one| word.eq_ignore_ascii_case(one)).ok_or_else(|| args.error_at(mark))
        };
        if keyword(args, &["delay", "clear"])? == 1 {
            return Ok(Self::Clear { function: args.word()?.to_owned() });
        }
        if keyword(args, &["add", "clear"])? == 1 {
            return Ok(Self::Clear { function: args.word()?.to_owned() });
        }
        let function = args.word()?.to_owned();
        let mark = args.mark();
        let ticks = ticks(args.word()?).ok_or_else(|| args.error_at(mark))?;
        let append = if args.is_empty() { false } else { keyword(args, &["replace", "append"])? == 1 };
        Ok(Self::Add { function, ticks, append })
    }
}

impl Command for Schedule {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let (Self::Add { function, .. } | Self::Clear { function }) = &self;
        if run::find(ctx, function).is_none() {
            return out.translate(NOT_FOUND, vec![function.clone()], false);
        }
        match self {
            Self::Add { ticks, .. } if ticks < 0 => out.translate("commands.schedule.delay.negativeTime", Vec::new(), false),
            Self::Add { function, ticks: 0, .. } => run_scheduled(ctx, &function),
            Self::Add { function, ticks, append } => {
                let (due, world) = (ctx.tick() + ticks as u64, ctx.world());
                let pending = &mut ctx.state::<Scheduled>().0;
                if !append {
                    pending.retain(|one| one.function != function);
                }
                pending.push(Pending { due, function: function.clone(), world });
                out.translate("commands.schedule.delay.functionQueued", vec![function, ticks.to_string(), due.to_string()], true);
            }
            Self::Clear { function } => {
                let pending = &mut ctx.state::<Scheduled>().0;
                let before = pending.len();
                pending.retain(|one| one.function != function);
                match before - pending.len() {
                    0 => out.translate("commands.schedule.clear.failure", vec![function], false),
                    cleared => out.translate("commands.schedule.clear.success", vec![cleared.to_string(), function], true),
                }
            }
        }
    }
}

/// Runs a scheduled function as the server and tells the operators, as BDS echoes a command:
/// `[Server: Function x ran n commands]`.
fn run_scheduled(ctx: &mut ShardCtx<'_>, function: &str) {
    let Called::Ran(lines) = run::call(ctx, function, &Sender::Console) else { return };
    if matches!(ctx.game_rule("sendCommandFeedback"), Some((_, RuleValue::Bool(false)))) {
        return;
    }
    let translate = |key: &str, with: Option<RawText>| RawPart::Translate { key: key.into(), with };
    let ran = RawText(vec![RawPart::Text(lines.to_string()), RawPart::Text(function.to_owned())]);
    let echo = RawText(vec![RawPart::Text("§7§o[".into()), translate("commands.origin.server", None), RawPart::Text(": ".into()), translate("commands.schedule.functionRan", Some(ran)), RawPart::Text("]".into())]);
    let operators: Vec<_> = ctx.players().filter(|player| ctx.permission(player.id) == Some(Permission::Operator)).collect();
    operators.iter().for_each(|player| ctx.tell_raw(player.id, &echo));
}

/// Every tick: the scheduled functions now due, then each of `tick.json`'s in every world here.
pub(super) fn tick(ctx: &mut ShardCtx<'_>) {
    let now = ctx.tick();
    let pending = std::mem::take(&mut ctx.state::<Scheduled>().0);
    let (due, waiting): (Vec<Pending>, Vec<Pending>) = pending.into_iter().partition(|one| one.due <= now);
    ctx.state::<Scheduled>().0 = waiting;
    for one in due {
        ctx.within(&one.world, |ctx| run_scheduled(ctx, &one.function));
    }
    let every_tick = ctx.state::<FunctionState>().tick_functions();
    if every_tick.is_empty() {
        return;
    }
    for world in ctx.worlds() {
        ctx.within(&world, |ctx| {
            for function in &every_tick {
                run::call(ctx, function, &Sender::Console);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_delay_reads_in_ticks_seconds_or_days() {
        assert_eq!([ticks("40"), ticks("2s"), ticks("1d"), ticks("1t"), ticks("-1"), ticks("x"), ticks("s")], [Some(40), Some(40), Some(24_000), Some(1), Some(-1), None, None]);
        let sender = Sender::Console;
        let parsed = |line: &str| Schedule::parse(&mut Args::new(line, 0, &sender, &[])).ok().map(|schedule| match schedule {
            Schedule::Add { function, ticks, append } => format!("add {function} {ticks} {append}"),
            Schedule::Clear { function } => format!("clear {function}"),
        });
        assert_eq!(parsed("delay add probe/later 2s append").as_deref(), Some("add probe/later 40 true"));
        assert_eq!(parsed("delay add probe/later 20").as_deref(), Some("add probe/later 20 false"));
        assert_eq!(parsed("delay clear probe/later").as_deref(), Some("clear probe/later"));
        assert_eq!(parsed("clear probe/later").as_deref(), Some("clear probe/later"));
        assert_eq!([parsed("delay add probe/later"), parsed("on_area_loaded add 0 0 0 1 1 1 f"), parsed("delay add f 20 later")], [None, None, None]);
    }
}
