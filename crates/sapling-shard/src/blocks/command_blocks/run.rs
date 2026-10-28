//! A command block's run and the chain after it, by BDS's `CommandBlock::tick`, `_executeChain`,
//! `_executeChainBlock` and `BaseCommandBlock::performCommand` (docs/research/command-blocks.md).

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::command::{Executed, ParseError};
use sapling_api::{BlockPos, Output, Permission, Sender, ShardCtx};

use super::{Fields, Kind, Shape, shape};
use crate::blocks::queued::{self, Job};
use crate::blocks::{container_changed, level, opposite};
use crate::components::Tick;
use crate::handlers::Ctx;
use crate::plugins::CommandRegistry;
use crate::{rules, worlds};

const ENABLED: &str = "commandBlocksEnabled";
const OUTPUT: &str = "commandBlockOutput";
const MAX_CHAIN: &str = "maxCommandChainLength";
/// What BDS refused a command block, which runs at the game directors' level: these answered
/// `commands.generic.error.permissions` (research/captures/bds-cmdlinks.txt, bds-cmdlinks2.txt;
/// `kick` ran). sapling's ban commands, which BDS has not, are refused with them.
const REFUSED: &[&str] = &["op", "deop", "allowlist", "whitelist", "setmaxplayers", "transfer", "place", "ban", "ban-ip", "pardon", "pardon-ip"];

/// Marks the block's condition and queues its run: at once for one that runs on its first tick
/// or waits no ticks, else after its delay. BDS queued it as its circuit was evaluated, a tick
/// before sapling applies what the evaluation found (redstone.rs), so it waits a tick less:
/// an impulse block waiting 5 ran 6 ticks after the block that powered it was put, one waiting
/// none 2 (bds-cmdchains.txt).
pub(super) fn start(world: &mut World, layer: Entity, pos: BlockPos, shape: &Shape) {
    let Some(mut fields) = super::read(world, layer, pos) else { return };
    mark_condition(world, layer, pos, shape, &mut fields);
    let delay = if fields.flag("ExecuteOnFirstTick") { 0 } else { fields.int("TickDelay").max(0) as u64 };
    fields.keep(world, layer, pos);
    match delay {
        0 | 1 => queued::this_tick(world, layer, pos, Job::Command),
        delay => queued::after(world, layer, pos, delay - 1, Job::Command),
    }
}

/// `conditionMet`: an unconditional block's always is; a conditional one's is where the block
/// behind it is a command block whose last run succeeded.
fn mark_condition(world: &World, layer: Entity, pos: BlockPos, shape: &Shape, fields: &mut Fields) {
    let behind = opposite(shape.facing).neighbour(pos);
    let met = !shape.conditional || super::shape(world, layer, behind).is_some() && super::read(world, layer, behind).is_some_and(|behind| behind.int("SuccessCount") > 0);
    fields.set_flag("conditionMet", met);
}

/// A queued run: an impulse or repeating block runs where its condition is met and then its
/// chain, a repeating one queuing its next run while powered or always active; a chain block
/// that waited its delay runs and goes on with its chain.
pub(crate) fn tick(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(shape) = shape(world, layer, pos) else { return };
    if shape.kind == Kind::Chain {
        if link(world, layer, pos, &shape, true) {
            chain(world, layer, pos);
        }
        return;
    }
    let Some(mut fields) = super::read(world, layer, pos) else { return };
    if shape.kind == Kind::Repeating {
        mark_condition(world, layer, pos, &shape, &mut fields);
    }
    let again = shape.kind == Kind::Repeating && (fields.flag("powered") || fields.flag("auto"));
    let delay = fields.int("TickDelay").max(0) as u64;
    let met = fields.flag("conditionMet");
    if !met && shape.conditional {
        fields.put("SuccessCount", Value::Int(0));
    }
    fields.keep(world, layer, pos);
    if met {
        perform(world, layer, pos);
        chain(world, layer, pos);
    }
    if again {
        queued::after(world, layer, pos, delay, Job::Command);
    }
}

/// Runs the chain blocks `from` faces into, one after another, up to `maxCommandChainLength` of
/// them: BDS ran none with the rule at -1, two at 2 (bds-cmdrules.txt). A block that is no chain
/// block, waits a delay or runs nothing ends it.
fn chain(world: &mut World, layer: Entity, from: BlockPos) {
    let limit = rules::number(world, layer, MAX_CHAIN).unwrap_or(i32::from(u16::MAX));
    let mut at = from;
    for _ in 0..limit.max(0) {
        let Some(here) = shape(world, layer, at) else { return };
        let next = here.facing.neighbour(at);
        let Some(link_shape) = shape(world, layer, next).filter(|next| next.kind == Kind::Chain) else { return };
        if !link(world, layer, next, &link_shape, false) {
            return;
        }
        at = next;
    }
}

/// One chain block's turn; whether the chain goes on. One that needs power and has none is
/// passed over; one with a delay is queued and ends the chain until it runs.
fn link(world: &mut World, layer: Entity, pos: BlockPos, shape: &Shape, from_tick: bool) -> bool {
    let Some(mut fields) = super::read(world, layer, pos) else { return false };
    if !fields.flag("powered") && !fields.flag("auto") {
        return true;
    }
    let delay = fields.int("TickDelay").max(0) as u64;
    if !from_tick && delay > 0 {
        queued::after(world, layer, pos, delay, Job::Command);
        return false;
    }
    mark_condition(world, layer, pos, shape, &mut fields);
    let met = fields.flag("conditionMet");
    if !met && shape.conditional {
        fields.put("SuccessCount", Value::Int(0));
    }
    fields.keep(world, layer, pos);
    !met || perform(world, layer, pos)
}

/// Runs the block's command, once a tick at most: false where it ran this tick already or
/// `commandBlocksEnabled` is off. It keeps the run's tick, `SuccessCount` (1 for a command that
/// answered nothing, as `say` and `daylock` counted on BDS) and, tracking output, the run's last
/// line; operators are shown what succeeded under `commandBlockOutput`, named `!` or the block's
/// name.
fn perform(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(mut fields) = super::read(world, layer, pos) else { return false };
    let now = world.resource::<Tick>().0 as i64;
    if fields.int("LastExecution") == now {
        return false;
    }
    fields.put("LastExecution", Value::Long(now));
    let line = fields.text("Command");
    let enabled = rules::on(world, layer, ENABLED);
    if !enabled || line.trim().is_empty() {
        fields.put("SuccessCount", Value::Int(0));
        fields.keep(world, layer, pos);
        container_changed(world, layer, pos);
        return enabled;
    }
    fields.keep(world, layer, pos);
    let out = run_line(world, layer, pos, &line);
    let Some(mut fields) = super::read(world, layer, pos) else { return true };
    let count = if out.messages().is_empty() { 1 } else { out.success_count() };
    fields.put("SuccessCount", Value::Int(i32::try_from(count).unwrap_or(i32::MAX)));
    if fields.flag("TrackOutput") {
        let (text, parameters) = out.messages().last().map_or((String::new(), Vec::new()), |last| (last.text.clone(), last.parameters.clone()));
        fields.set_output(&text, &parameters);
    }
    let name = match fields.text("CustomName") {
        named if !named.is_empty() => format!("{named}§r"),
        _ => "!§r".to_owned(),
    };
    fields.keep(world, layer, pos);
    container_changed(world, layer, pos);
    if rules::on(world, layer, OUTPUT) {
        crate::commands::echo_to_operators(world, None, &name, &out);
    }
    true
}

/// Runs `line` for the block at `pos`: from its middle, as the console under `/execute` would.
// TODO: BDS names the block `!` (or its name) where a command names its sender, as `/say` does;
// sapling's `Sender` has no variant for a block, so it is named as the console is.
fn run_line(world: &mut World, layer: Entity, pos: BlockPos, line: &str) -> Output {
    let line = line.trim();
    let line = line.strip_prefix('/').unwrap_or(line);
    let word = line.split_whitespace().next().unwrap_or("");
    if REFUSED.iter().any(|refused| refused.eq_ignore_ascii_case(word)) {
        let mut out = Output::default();
        out.translate("commands.generic.error.permissions", vec![word.to_owned()], false);
        return out;
    }
    let dimension = level(world, layer).map_or(0, |level| level.dimension());
    let sender = Sender::Executed(Box::new(Executed::new(Sender::Console, None, pos.map(|c| c as f32 + 0.5), [0.0, 0.0], dimension)));
    let commands = world.resource::<CommandRegistry>().0.clone();
    let focused = worlds::focused(world);
    worlds::focus(world, Some(layer));
    let out = commands.run(&mut ShardCtx::new(&mut Ctx { world: &mut *world }), &sender, Permission::Operator, line);
    worlds::focus(world, focused);
    crate::scoreboard::send(world);
    out
}

/// The output BDS kept for a command that does not parse, as it compiled the command on the
/// editor's update: `commands.generic.syntax` with where it broke. A target nobody matches is no
/// error yet.
pub(super) fn syntax_error(world: &World, line: &str) -> Option<(String, Vec<String>)> {
    if line.trim().is_empty() {
        return None;
    }
    match world.resource::<CommandRegistry>().0.check(line, Permission::Operator) {
        Ok(()) | Err(ParseError::NoTarget) => None,
        Err(error) => {
            let mut out = Output::default();
            out.refuse(error);
            out.messages().first().map(|message| (message.text.clone(), message.parameters.clone()))
        }
    }
}
