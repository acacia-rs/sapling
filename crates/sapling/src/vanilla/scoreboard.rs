//! `/scoreboard`, read as BDS's eleven overloads read it and answering as BDS did
//! (research/captures/bds-scoreboard.txt, bds-scoreboard2.txt): objectives.rs and players.rs.
// A number past i32 is a syntax error here; BDS answered `commands.generic.num.invalid`.

mod objectives;
mod players;

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector, parse_choice};
use sapling_api::{Command, DisplaySlot, Output, Permission, ScoreOperation, Sender, ShardCtx, SortOrder};

#[derive(Clone, Copy)]
pub(super) enum Change {
    Set,
    Add,
    Remove,
}

pub(crate) enum Scoreboard {
    Add { name: String, display_name: Option<String> },
    Remove(String),
    List,
    SetDisplay { slot: DisplaySlot, shown: Option<(String, SortOrder)> },
    ListScores(Option<Selector>),
    Reset { target: Selector, objective: Option<String> },
    Test { target: Selector, objective: String, min: i32, max: i32 },
    Random { target: Selector, objective: String, min: i32, max: i32 },
    Change { change: Change, target: Selector, objective: String, count: i32 },
    Operation { target: Selector, objective: String, operation: ScoreOperation, source: Selector, source_objective: String },
}

fn param(name: &str, kind: ParamKind, optional: bool) -> ParameterSpec {
    ParameterSpec { name: name.into(), kind, optional }
}

fn words(name: &str, values: &[&str]) -> ParamKind {
    ParamKind::Enum { name: name.into(), values: values.iter().map(|value| (*value).to_owned()).collect() }
}

/// BDS's overloads, its enum names too; wildcard numbers and the operator go as strings.
fn overloads() -> Vec<Overload> {
    let objectives = || param("category", words("ScoreboardObjectivesCategory", &["objectives"]), false);
    let players = || param("category", words("ScoreboardPlayersCategory", &["players"]), false);
    let action = |name: &str, values: &[&str]| param("action", words(name, values), false);
    let objective = |optional| param("objective", ParamKind::String, optional);
    let player = |name: &str, optional| param(name, ParamKind::Target, optional);
    let int = |name: &str| param(name, ParamKind::Int, false);
    let parameters: [Vec<ParameterSpec>; 11] = [
        vec![objectives(), action("ScoreboardAddAction", &["add"]), objective(false), param("criteria", words("ScoreboardCriteria", &["dummy"]), false), param("displayName", ParamKind::String, true)],
        vec![objectives(), action("ScoreboardRemoveAction", &["remove"]), objective(false)],
        vec![objectives(), action("ScoreboardListAction", &["list"])],
        vec![objectives(), action("ScoreboardSetDisplayAction", &["setdisplay"]), param("displaySlot", words("ScoreboardDisplaySlotSortable", &["list", "sidebar"]), false), objective(true), param("sortOrder", words("ScoreboardSortOrder", &["ascending", "descending"]), true)],
        vec![objectives(), action("ScoreboardSetDisplayAction", &["setdisplay"]), param("displaySlot", words("ScoreboardDisplaySlotNonSortable", &["belowname"]), false), objective(true)],
        vec![players(), action("ScoreboardListAction", &["list"]), player("playername", true)],
        vec![players(), action("ScoreboardResetAction", &["reset"]), player("player", false), objective(true)],
        vec![players(), action("ScoreboardTestAction", &["test"]), player("player", false), objective(false), param("min", ParamKind::String, false), param("max", ParamKind::String, true)],
        vec![players(), action("ScoreboardRandomAction", &["random"]), player("player", false), objective(false), int("min"), int("max")],
        vec![players(), action("ScoreboardPlayersNumAction", &["set", "add", "remove"]), player("player", false), objective(false), int("count")],
        vec![players(), action("ScoreboardOperationAction", &["operation"]), player("targetName", false), param("targetObjective", ParamKind::String, false), param("operation", ParamKind::String, false), player("selector", false), objective(false)],
    ];
    parameters.into_iter().map(|parameters| Overload { parameters }).collect()
}

/// A number, or `*` for `wildcard`.
fn bound(args: &mut Args<'_>, wildcard: i32) -> Result<i32, ParseError> {
    let mark = args.mark();
    match args.word()? {
        "*" => Ok(wildcard),
        word => word.parse().map_err(|_| args.error_at(mark)),
    }
}

fn objectives_part(args: &mut Args<'_>) -> Result<Scoreboard, ParseError> {
    Ok(match parse_choice(args, &[("add", 0), ("remove", 1), ("list", 2), ("setdisplay", 3)])? {
        0 => {
            let name = String::parse(args)?;
            parse_choice(args, &[("dummy", ())])?;
            Scoreboard::Add { name, display_name: if args.is_empty() { None } else { Some(String::parse(args)?) } }
        }
        1 => Scoreboard::Remove(String::parse(args)?),
        2 => Scoreboard::List,
        _ => {
            let slot = parse_choice(args, &DisplaySlot::ALL.map(|slot| (slot.name(), slot)))?;
            if args.is_empty() {
                return Ok(Scoreboard::SetDisplay { slot, shown: None });
            }
            let objective = String::parse(args)?;
            let sortable = slot != DisplaySlot::BelowName && !args.is_empty();
            let order = if sortable { parse_choice(args, &[("ascending", SortOrder::Ascending), ("descending", SortOrder::Descending)])? } else { SortOrder::Descending };
            Scoreboard::SetDisplay { slot, shown: Some((objective, order)) }
        }
    })
}

fn players_part(args: &mut Args<'_>) -> Result<Scoreboard, ParseError> {
    let actions = [("list", None), ("reset", None), ("test", None), ("random", None), ("operation", None), ("set", Some(Change::Set)), ("add", Some(Change::Add)), ("remove", Some(Change::Remove))];
    let mark = args.mark();
    let action = parse_choice(args, &actions.map(|(word, change)| (word, (word, change))))?;
    if action.0 == "list" {
        return Ok(Scoreboard::ListScores(if args.is_empty() { None } else { Some(Selector::parse(args)?) }));
    }
    let target = Selector::parse(args)?;
    if action.0 == "reset" {
        return Ok(Scoreboard::Reset { target, objective: if args.is_empty() { None } else { Some(String::parse(args)?) } });
    }
    let objective = String::parse(args)?;
    Ok(match action {
        ("test", _) => {
            let min = bound(args, i32::MIN)?;
            Scoreboard::Test { target, objective, min, max: if args.is_empty() { i32::MAX } else { bound(args, i32::MAX)? } }
        }
        ("random", _) => Scoreboard::Random { target, objective, min: i32::parse(args)?, max: i32::parse(args)? },
        ("operation", _) => {
            let operation = parse_choice(args, &ScoreOperation::SYMBOLS)?;
            Scoreboard::Operation { target, objective, operation, source: Selector::parse(args)?, source_objective: String::parse(args)? }
        }
        (_, Some(change)) => Scoreboard::Change { change, target, objective, count: i32::parse(args)? },
        _ => return Err(args.error_at(mark)),
    })
}

impl CommandSyntax for Scoreboard {
    fn spec() -> CommandSpec {
        CommandSpec { name: "scoreboard".into(), aliases: Vec::new(), description: "Tracks and displays scores for various objectives".into(), permission: Permission::Operator, overloads: overloads() }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let parsed = match parse_choice(args, &[("objectives", true), ("players", false)])? {
            true => objectives_part(args)?,
            false => players_part(args)?,
        };
        args.finish()?;
        Ok(parsed)
    }
}

impl Command for Scoreboard {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        match self {
            Self::Add { name, display_name } => objectives::add(ctx, out, name, display_name),
            Self::Remove(name) => objectives::remove(ctx, out, &name),
            Self::List => objectives::list(ctx, out),
            Self::SetDisplay { slot, shown } => objectives::set_display(ctx, out, slot, shown),
            Self::ListScores(target) => players::list(ctx, sender, out, target.as_ref()),
            Self::Reset { target, objective } => players::reset(ctx, sender, out, &target, objective.as_deref()),
            Self::Test { target, objective, min, max } => players::test(ctx, sender, out, &target, &objective, [min, max]),
            Self::Random { target, objective, min, max } => players::random(ctx, sender, out, &target, &objective, [min, max]),
            Self::Change { change, target, objective, count } => players::change(ctx, sender, out, change, &target, &objective, count),
            Self::Operation { target, objective, operation, source, source_objective } => players::operation(ctx, sender, out, (&target, &objective), operation, (&source, &source_objective)),
        }
    }
}

/// `objectiveNotFound` unless `name` is an objective.
fn find_objective(ctx: &ShardCtx<'_>, out: &mut Output, name: &str) -> bool {
    let found = ctx.objective(name).is_some();
    if !found {
        out.translate("commands.scoreboard.objectiveNotFound", vec![name.to_owned()], false);
    }
    found
}

#[cfg(test)]
mod tests;
