//! `/inputpermission`, `/mobevent` and `/daylock` (`/alwaysday`), answered as BDS 1.26.52 does
//! (research/captures/bds-controls.txt, bds-controls2.txt, bds-screen.txt).
// TODO: `/mobevent` only keeps its switches: sapling spawns no patrols, wandering traders or
// dragons of its own for them to stop.

use std::collections::HashMap;

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Input, Overload, ParamKind, Parameter, ParameterSpec, ParseError, RawPart, RawText, Selector};
use sapling_api::{Command, Output, Permission, RuleValue, Sender, ShardCtx};

use super::{no_target, players};

/// How BDS's answers name each input, spaces and all (a trailing one doubles the space after).
const SHOWN: [&str; 11] = ["Camera", "Movement", "Lateral Movement ", "Sneak", "Jump", "Mount ", "Dismount ", "Move Forward ", "Move Backward", "Move Left", "Move Right"];

fn shown(input: Input) -> &'static str {
    Input::ALL.iter().position(|known| *known == input).map_or("", |at| SHOWN[at])
}

fn enum_param(name: &str, values: &[&str], optional: bool) -> ParameterSpec {
    ParameterSpec { name: name.into(), kind: ParamKind::Enum { name: name.into(), values: values.iter().map(|value| (*value).to_owned()).collect() }, optional }
}

/// `/inputpermission query <targets> <permission> [state]` and `set <targets> <permission> <state>`.
/// Its answers are plain text, not translation keys.
pub(crate) struct InputPermission {
    set: bool,
    targets: Selector,
    input: Input,
    enabled: Option<bool>,
}

fn state(args: &mut Args<'_>) -> Result<bool, ParseError> {
    let mark = args.mark();
    match args.word()?.to_ascii_lowercase().as_str() {
        "enabled" => Ok(true),
        "disabled" => Ok(false),
        _ => Err(args.error_at(mark)),
    }
}

impl CommandSyntax for InputPermission {
    fn spec() -> CommandSpec {
        let names: Vec<&str> = Input::ALL.iter().map(|input| input.name()).collect();
        let states = ["enabled", "disabled"];
        let common = |action: &str| vec![ParameterSpec::literal("inputpermission", action), ParameterSpec::of::<Selector>("targets"), enum_param("permission", &names, false)];
        let query = common("query").into_iter().chain([enum_param("state", &states, true)]).collect();
        let set = common("set").into_iter().chain([enum_param("state", &states, false)]).collect();
        let overloads = vec![Overload { parameters: query }, Overload { parameters: set }];
        CommandSpec { name: "inputpermission".into(), aliases: Vec::new(), description: "Sets whether or not a player's input can affect their character.".into(), permission: Permission::Operator, overloads }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let set = match args.word()?.to_ascii_lowercase().as_str() {
            "set" => true,
            "query" => false,
            _ => return Err(args.error_at(mark)),
        };
        let targets = Selector::parse(args)?;
        let mark = args.mark();
        let input = Input::named(&args.word()?.to_ascii_lowercase()).ok_or_else(|| args.error_at(mark))?;
        let enabled = if set || !args.is_empty() { Some(state(args)?) } else { None };
        args.finish()?;
        Ok(Self { set, targets, input, enabled })
    }
}

impl Command for InputPermission {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let found = players(ctx, sender, Some(&self.targets));
        let name = shown(self.input);
        let word = |enabled: bool| if enabled { "enabled" } else { "disabled" };
        let allowed: Vec<bool> = found.iter().map(|player| ctx.input_allowed(player.id, self.input)).collect();
        let named = |which: bool| found.iter().zip(&allowed).filter(|(_, allowed)| **allowed == which).map(|(player, _)| player.name.as_str()).collect::<Vec<_>>();
        match (self.set, self.enabled) {
            (true, Some(enabled)) => {
                if found.is_empty() {
                    return no_target(out);
                }
                found.iter().for_each(|player| ctx.allow_input(player.id, self.input, enabled));
                let whom = if let [one] = found.as_slice() { one.name.clone() } else { format!("{} players", found.len()) };
                out.translate(format!("{name} input {} for {whom}  ", word(enabled)), vec![found.len().to_string()], true);
            }
            (_, Some(enabled)) => {
                let matching = named(enabled);
                out.translate(format!("<{name}>: {} {}", matching.len(), word(enabled)), vec![matching.join(", ")], true);
                // BDS counted a success only where every target was in the state asked about.
                out.done_to(u32::from(matching.len() == found.len()));
            }
            (_, None) => {
                let (on, off) = (named(true), named(false));
                out.translate(format!("<{name}>: {} enabled | {} disabled", on.len(), off.len()), vec![on.join(", ")], true);
            }
        }
    }
}

/// The events `/mobevent` switches, and the entity each answer names.
const EVENTS: [(&str, &str); 3] = [("minecraft:pillager_patrols_event", "pillager"), ("minecraft:wandering_trader_event", "wandering_trader"), ("minecraft:ender_dragon_event", "ender_dragon")];
const ALL_EVENTS: &str = "events_enabled";

/// Which `/mobevent` switches are off; every one starts on.
#[derive(Default)]
struct MobEvents(HashMap<&'static str, bool>);

/// `/mobevent <event> [value]`: shows or sets one of the switches.
pub(crate) struct MobEvent {
    event: &'static str,
    value: Option<bool>,
}

impl CommandSyntax for MobEvent {
    fn spec() -> CommandSpec {
        let names: Vec<&str> = EVENTS.iter().map(|(event, _)| *event).chain([ALL_EVENTS]).collect();
        let parameters = vec![enum_param("event", &names, false), ParameterSpec::of::<Option<bool>>("value")];
        CommandSpec { name: "mobevent".into(), aliases: Vec::new(), description: "Controls what mob events are allowed to run.".into(), permission: Permission::Operator, overloads: vec![Overload { parameters }] }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let word = args.word()?.to_ascii_lowercase();
        let event = EVENTS.iter().map(|(event, _)| *event).chain([ALL_EVENTS]).find(|event| *event == word).ok_or_else(|| args.error_at(mark))?;
        let value = Option::<bool>::parse(args)?;
        args.finish()?;
        Ok(Self { event, value })
    }
}

impl Command for MobEvent {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let switches = &mut ctx.state::<MobEvents>().0;
        let now = self.value.unwrap_or_else(|| *switches.get(self.event).unwrap_or(&true));
        if let Some(value) = self.value {
            switches.insert(self.event, value);
        }
        let told = match (self.value.is_some(), now) {
            (false, true) => "IsTrue",
            (false, false) => "IsFalse",
            (true, true) => "SetToTrue",
            (true, false) => "SetToFalse",
        };
        match EVENTS.iter().find(|(event, _)| *event == self.event) {
            Some((event, entity)) => out.translate(format!("commands.mobevent.event{told}"), vec![format!("%entity.{entity}.name"), (*event).to_owned()], true),
            None => out.translate(format!("commands.mobevent.eventsEnabled{told}"), Vec::new(), true),
        }
    }
}

const DAY: i64 = 24000;
const NOON: i64 = 6000;

/// `/daylock [lock]` (`/alwaysday`): stops the day at noon, or lets it run again. BDS answers in
/// chat to every player, with no command output; locking moves the time to the next noon, as
/// `/time set noon` does, only where the day was running.
#[derive(Command)]
#[command(crate = "crate", name = "daylock", aliases = ["alwaysday"], description = "Locks and unlocks the day-night cycle.", permission = "operator")]
pub(crate) struct DayLock {
    lock: Option<bool>,
}

impl Command for DayLock {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, _: &mut Output) {
        let lock = self.lock.unwrap_or(true);
        let running = !matches!(ctx.game_rule(sapling_data::DAYLIGHT_CYCLE), Some((_, RuleValue::Bool(false))));
        if lock && running {
            let now = ctx.time();
            let noon = now.div_euclid(DAY) * DAY + NOON;
            ctx.set_time(if noon < now { noon + DAY } else { noon });
        }
        if lock == running {
            ctx.set_game_rule(sapling_data::DAYLIGHT_CYCLE, RuleValue::Bool(!lock));
        }
        let key = if lock { "commands.always.day.locked" } else { "commands.always.day.unlocked" };
        let text = RawText(vec![RawPart::Translate { key: key.into(), with: Some(RawText(Vec::new())) }]);
        for player in ctx.players().collect::<Vec<_>>() {
            ctx.tell_raw(player.id, &text);
        }
    }
}
