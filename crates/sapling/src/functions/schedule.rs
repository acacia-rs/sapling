//! `/schedule delay add|clear`, `/schedule on_area_loaded add` and what runs every tick, as BDS
//! answered and ran them (research/captures/bds-functions.txt, bds-u3sched.txt): a function is run
//! in the world it was scheduled in, once its delay (`20`, `2s`, `1d`) has passed by the server, or
//! once every chunk of its area ticks by its sender, and the operators are told; a delay of 0 or an
//! area that ticks already runs it at once. An area is a box, a circle of chunks, or a ticking area
//! by name, which need not exist yet. `clear` takes both kinds.

use sapling_api::command::{Args, BlockCoords, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, RawPart, RawText};
use sapling_api::{Command, Output, Permission, RuleValue, Sender, ShardCtx, TickingArea};

use super::commands::{NOT_FOUND, function_parameter};
use super::run::{self, Called, FunctionState};

const TICKS_PER_SECOND: i64 = 20;
const TICKS_PER_DAY: i64 = 24_000;

struct Pending {
    due: u64,
    function: String,
    world: String,
}

/// A function waiting for its area to tick, and who scheduled it.
struct Waiting {
    area: Area,
    function: String,
    world: String,
    sender: Sender,
}

/// The functions waiting for their delay or area on this shard, in the order they were scheduled.
#[derive(Default)]
struct Scheduled {
    delays: Vec<Pending>,
    areas: Vec<Waiting>,
}

/// The chunks a function waits on: these, or those of the ticking area of that name.
enum Area {
    Chunks(TickingArea),
    Named(String),
}

/// An area as the command names it.
#[derive(Debug, PartialEq)]
pub(crate) enum Place {
    Box(BlockCoords, BlockCoords),
    Circle(BlockCoords, i32),
    Named(String),
}

pub(crate) enum Schedule {
    Add { function: String, ticks: i64, append: bool },
    OnLoaded { place: Place, function: String },
    Clear { function: String, areas: bool },
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
        let at = |name: &str| ParameterSpec::of::<BlockCoords>(name);
        let name = ParameterSpec { name: "tickingAreaName".into(), kind: ParamKind::String, optional: false };
        let loaded = || vec![word("schedule", "on_area_loaded"), word("schedule", "add")];
        let overloads = [
            vec![word("schedule", "delay"), word("schedule", "add"), function_parameter("function"), time, mode],
            vec![word("schedule", "delay"), word("schedule", "clear"), function_parameter("function")],
            vec![word("schedule", "clear"), function_parameter("function")],
            [loaded(), vec![at("from"), at("to"), function_parameter("function")]].concat(),
            [loaded(), vec![word("schedule", "circle"), at("center"), ParameterSpec::of::<i32>("radius"), function_parameter("function")]].concat(),
            [loaded(), vec![word("schedule", "tickingarea"), name, function_parameter("function")]].concat(),
        ];
        let overloads = overloads.into_iter().map(|parameters| Overload { parameters }).collect();
        CommandSpec { name: "schedule".into(), aliases: Vec::new(), description: "Schedules an action to be executed once an area is loaded, or after a certain amount of time.".into(), permission: Permission::Operator, overloads }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let keyword = |args: &mut Args<'_>, wanted: &[&str]| -> Result<usize, ParseError> {
            let mark = args.mark();
            let word = args.word()?;
            wanted.iter().position(|one| word.eq_ignore_ascii_case(one)).ok_or_else(|| args.error_at(mark))
        };
        match keyword(args, &["delay", "clear", "on_area_loaded"])? {
            1 => return Ok(Self::Clear { function: args.word()?.to_owned(), areas: true }),
            2 => {
                keyword(args, &["add"])?;
                let place = match args.peek() {
                    Some(word) if word.eq_ignore_ascii_case("circle") => {
                        args.word()?;
                        Place::Circle(BlockCoords::parse(args)?, i32::parse(args)?)
                    }
                    Some(word) if word.eq_ignore_ascii_case("tickingarea") => {
                        args.word()?;
                        Place::Named(args.word()?.to_owned())
                    }
                    _ => Place::Box(BlockCoords::parse(args)?, BlockCoords::parse(args)?),
                };
                let function = args.word()?.to_owned();
                args.finish()?;
                return Ok(Self::OnLoaded { place, function });
            }
            _ => {}
        }
        if keyword(args, &["add", "clear"])? == 1 {
            return Ok(Self::Clear { function: args.word()?.to_owned(), areas: false });
        }
        let function = args.word()?.to_owned();
        let mark = args.mark();
        let ticks = ticks(args.word()?).ok_or_else(|| args.error_at(mark))?;
        let append = if args.is_empty() { false } else { keyword(args, &["replace", "append"])? == 1 };
        Ok(Self::Add { function, ticks, append })
    }
}

/// The chunks of `place` seen from `sender`.
fn area(ctx: &ShardCtx<'_>, sender: &Sender, place: Place) -> Area {
    let chunk = |at: &BlockCoords| {
        let [x, _, z] = at.resolve(ctx, sender);
        [x >> 4, z >> 4]
    };
    let chunks = |min: [i32; 2], max: [i32; 2], circle| Area::Chunks(TickingArea { name: String::new(), min, max, circle, preload: false });
    match place {
        Place::Box(from, to) => {
            let (from, to) = (chunk(&from), chunk(&to));
            chunks([from[0].min(to[0]), from[1].min(to[1])], [from[0].max(to[0]), from[1].max(to[1])], false)
        }
        Place::Circle(centre, radius) => {
            let [x, z] = chunk(&centre);
            let radius = radius.max(0);
            chunks([x - radius, z - radius], [x + radius, z + radius], true)
        }
        Place::Named(name) => Area::Named(name),
    }
}

/// Whether every chunk of `area` ticks in the world in scope; a named area must exist.
fn ticks_now(ctx: &mut ShardCtx<'_>, area: &Area) -> bool {
    let named;
    let chunks = match area {
        Area::Chunks(chunks) => chunks,
        Area::Named(name) => match ctx.ticking_areas().into_iter().find(|one| one.name == *name) {
            Some(found) => {
                named = found;
                &named
            }
            None => return false,
        },
    };
    let all: Vec<[i32; 2]> = (chunks.min[0]..=chunks.max[0]).flat_map(|x| (chunks.min[1]..=chunks.max[1]).map(move |z| [x, z])).filter(|chunk| chunks.holds(*chunk)).collect();
    all.into_iter().all(|chunk| ctx.ticks_chunk(chunk))
}

impl Command for Schedule {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let (Self::Add { function, .. } | Self::OnLoaded { function, .. } | Self::Clear { function, .. }) = &self;
        if run::find(ctx, function).is_none() {
            return out.translate(NOT_FOUND, vec![function.clone()], false);
        }
        match self {
            Self::Add { ticks, .. } if ticks < 0 => out.translate("commands.schedule.delay.negativeTime", Vec::new(), false),
            Self::Add { function, ticks: 0, .. } => run_scheduled(ctx, &function, &Sender::Console),
            Self::Add { function, ticks, append } => {
                let (due, world) = (ctx.tick() + ticks as u64, ctx.world());
                let pending = &mut ctx.state::<Scheduled>().delays;
                if !append {
                    pending.retain(|one| one.function != function);
                }
                pending.push(Pending { due, function: function.clone(), world });
                out.translate("commands.schedule.delay.functionQueued", vec![function, ticks.to_string(), due.to_string()], true);
            }
            Self::OnLoaded { place, function } => {
                let area = area(ctx, sender, place);
                if ticks_now(ctx, &area) {
                    return run_scheduled(ctx, &function, sender);
                }
                let world = ctx.world();
                ctx.state::<Scheduled>().areas.push(Waiting { area, function, world, sender: sender.clone() });
                out.translate("commands.schedule.functionQueued", Vec::new(), true);
            }
            Self::Clear { function, areas } => {
                let scheduled = ctx.state::<Scheduled>();
                let before = scheduled.delays.len() + scheduled.areas.len();
                scheduled.delays.retain(|one| one.function != function);
                if areas {
                    scheduled.areas.retain(|one| one.function != function);
                }
                match before - scheduled.delays.len() - scheduled.areas.len() {
                    0 => out.translate("commands.schedule.clear.failure", vec![function], false),
                    cleared => out.translate("commands.schedule.clear.success", vec![cleared.to_string(), function], true),
                }
            }
        }
    }
}

/// Runs a scheduled function as `sender` and tells the operators as the server, as BDS echoes a
/// command: `[Server: Function x ran n commands]`.
fn run_scheduled(ctx: &mut ShardCtx<'_>, function: &str, sender: &Sender) {
    let Called::Ran(lines) = run::call(ctx, function, sender) else { return };
    if matches!(ctx.game_rule("sendCommandFeedback"), Some((_, RuleValue::Bool(false)))) {
        return;
    }
    let translate = |key: &str, with: Option<RawText>| RawPart::Translate { key: key.into(), with };
    let ran = RawText(vec![RawPart::Text(lines.to_string()), RawPart::Text(function.to_owned())]);
    let echo = RawText(vec![RawPart::Text("§7§o[".into()), translate("commands.origin.server", None), RawPart::Text(": ".into()), translate("commands.schedule.functionRan", Some(ran)), RawPart::Text("]".into())]);
    let operators: Vec<_> = ctx.players().filter(|player| ctx.permission(player.id) == Some(Permission::Operator)).collect();
    operators.iter().for_each(|player| ctx.show_raw(player.id, &echo));
}

/// Every tick: the scheduled functions now due or whose area now ticks, then each of `tick.json`'s
/// in every world here.
pub(super) fn tick(ctx: &mut ShardCtx<'_>) {
    let now = ctx.tick();
    let pending = std::mem::take(&mut ctx.state::<Scheduled>().delays);
    let (due, later): (Vec<Pending>, Vec<Pending>) = pending.into_iter().partition(|one| one.due <= now);
    ctx.state::<Scheduled>().delays = later;
    for one in due {
        ctx.within(&one.world, |ctx| run_scheduled(ctx, &one.function, &Sender::Console));
    }
    let waiting = std::mem::take(&mut ctx.state::<Scheduled>().areas);
    let mut still = Vec::new();
    for one in waiting {
        if ctx.within(&one.world, |ctx| ticks_now(ctx, &one.area)) == Some(true) {
            ctx.within(&one.world, |ctx| run_scheduled(ctx, &one.function, &one.sender));
        } else {
            still.push(one);
        }
    }
    // Scheduled by a function that ran just now: after those already waiting.
    ctx.state::<Scheduled>().areas.splice(0..0, still);
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
    fn a_delay_reads_in_ticks_seconds_or_days_and_an_area_as_a_box_circle_or_name() {
        assert_eq!([ticks("40"), ticks("2s"), ticks("1d"), ticks("1t"), ticks("-1"), ticks("x"), ticks("s")], [Some(40), Some(40), Some(24_000), Some(1), Some(-1), None, None]);
        let sender = Sender::Console;
        let parsed = |line: &str| Schedule::parse(&mut Args::new(line, 0, &sender, &[])).ok().map(|schedule| match schedule {
            Schedule::Add { function, ticks, append } => format!("add {function} {ticks} {append}"),
            Schedule::OnLoaded { place, function } => format!("{place:?} {function}"),
            Schedule::Clear { function, areas } => format!("clear {function} {areas}"),
        });
        assert_eq!(parsed("delay add probe/later 2s append").as_deref(), Some("add probe/later 40 true"));
        assert_eq!(parsed("delay add probe/later 20").as_deref(), Some("add probe/later 20 false"));
        assert_eq!(parsed("delay clear probe/later").as_deref(), Some("clear probe/later false"));
        assert_eq!(parsed("clear probe/later").as_deref(), Some("clear probe/later true"));
        assert!(parsed("on_area_loaded add 0 0 0 1 1 1 f").is_some_and(|line| line.starts_with("Box(") && line.ends_with(" f")));
        assert!(parsed("on_area_loaded add circle 5000 0 5000 2 f").is_some_and(|line| line.starts_with("Circle(") && line.ends_with(", 2) f")));
        assert_eq!(parsed("on_area_loaded add tickingarea u3area f").as_deref(), Some("Named(\"u3area\") f"));
        assert_eq!([parsed("delay add probe/later"), parsed("on_area_loaded add 0 0 0 1 1 f"), parsed("delay add f 20 later")], [None, None, None]);
    }
}
