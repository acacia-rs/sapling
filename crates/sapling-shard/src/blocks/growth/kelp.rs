//! Kelp grows on a tick it schedules itself on BDS's random-tick queue, not by a random tick: a
//! wait drawn to match the Java game's 14% a random tick at a speed of 3 (`KelpBlock` 142fdc460),
//! on a queue that runs `randomTickSpeed` steps a tick. BDS's kelp at 200 put on a cell every 48
//! ticks or so (research/captures/bds-plants.txt), the drawn wait's mean of 9750 steps over 200.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_data::RuleValue;

use super::super::{level, send};
use super::{Picks, RULE};

const KELP: &str = "minecraft:kelp";
const AGE: &str = "kelp_age";
const OLDEST: u8 = 25;
/// The chance a random tick would grow it, and the speed the wait is drawn for.
const ODDS: f64 = 0.14;
const DRAWN_SPEED: f64 = 3.0;
const CELLS: f64 = 4096.0;

/// Each world's random-tick queue: its clock and the kelp due on it.
#[derive(Resource, Default)]
pub(crate) struct Kelp {
    clock: HashMap<Entity, u64>,
    due: Vec<(u64, Entity, BlockPos)>,
    /// Kelp written since the last tick, whose wait is drawn then.
    fresh: Vec<(Entity, BlockPos)>,
}

fn age_at(level: &sapling_world::World, [x, y, z]: BlockPos) -> Option<u8> {
    let state = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().filter(|state| state.name == KELP)?;
    state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(key, _)| *key == AGE).and_then(|(_, value)| value.parse().ok())
}

fn is_source(level: &sapling_world::World, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.is_loaded(send::column([x, y, z])) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.name == "minecraft:water" && state.liquid_depth == 0)
}

/// Steps of the queue to the next growth: a geometric wait (`floor(ln u / ln(1 - p)) + 1`).
fn wait(picks: &mut Picks) -> u64 {
    let draw = (f64::from(picks.below(1 << 24)) + 1.0) / f64::from(1u32 << 24);
    (draw.ln() / (1.0 - ODDS * DRAWN_SPEED / CELLS).ln()).floor() as u64 + 1
}

impl Kelp {
    /// Queues the kelp at `pos` to grow after `steps` of its world's queue, unless it waits already.
    fn queue(&mut self, layer: Entity, pos: BlockPos, steps: u64) {
        if !self.due.iter().any(|(_, at_layer, at)| (*at_layer, *at) == (layer, pos)) {
            let now = self.clock.get(&layer).copied().unwrap_or(0);
            self.due.push((now + steps, layer, pos));
        }
    }
}

/// Kelp written at `pos`, or found at the top of a stalk in a column that came in, waits to grow.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    if level(world, layer).and_then(|level| age_at(level, pos)).is_some() {
        world.get_resource_or_insert_with(Kelp::default).fresh.push((layer, pos));
    }
}

/// The tops of the stalks among `cells`, a column's kelp that came in.
pub(super) fn arrived(world: &mut World, layer: Entity, cells: &[BlockPos]) {
    let Some(level) = level(world, layer) else { return };
    let tops: Vec<(Entity, BlockPos)> = cells.iter().filter(|pos| age_at(level, Face::Up.neighbour(**pos)).is_none()).map(|pos| (layer, *pos)).collect();
    if !tops.is_empty() {
        world.get_resource_or_insert_with(Kelp::default).fresh.extend(tops);
    }
}

/// Bone meal on kelp: the top of its stalk, if it waits to grow, grows the next tick instead
/// (142fdcbe0); false where nothing waited.
pub(crate) fn meal(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(level) = level(world, layer) else { return false };
    let top = (0..).map(|up| [pos[0], pos[1] + up, pos[2]]).take_while(|at| age_at(level, *at).is_some()).last().unwrap_or(pos);
    let Some(mut kelp) = world.get_resource_mut::<Kelp>() else { return false };
    let now = kelp.clock.get(&layer).copied().unwrap_or(0);
    let Some(entry) = kelp.due.iter_mut().find(|(_, at_layer, at)| (*at_layer, *at) == (layer, top)) else { return false };
    entry.0 = now;
    true
}

/// Runs each world's queue on by its `randomTickSpeed` and grows the kelp due: a cell up into a
/// water source over it, a year older, which waits in turn.
pub(super) fn tick(world: &mut World, picks: &mut Picks) {
    let Some(mut kelp) = world.remove_resource::<Kelp>() else { return };
    for (layer, pos) in std::mem::take(&mut kelp.fresh) {
        let steps = wait(picks);
        kelp.queue(layer, pos, steps);
    }
    let mut layers: Vec<Entity> = kelp.due.iter().map(|(_, layer, _)| *layer).collect();
    layers.sort_unstable();
    layers.dedup();
    for layer in layers {
        let speed = match level(world, layer).and_then(|level| level.level().rules.get(RULE)) {
            Some((_, RuleValue::Int(speed))) => speed.max(0) as u64,
            _ => 0,
        };
        *kelp.clock.entry(layer).or_default() += speed;
    }
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut kelp.due).into_iter().partition(|(when, layer, _)| kelp.clock.get(layer).is_some_and(|now| when <= now));
    kelp.due = waiting;
    world.insert_resource(kelp);
    for (_, layer, pos) in due {
        grow(world, layer, pos);
    }
}

/// The new head is written, which queues it in turn.
fn grow(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let Some(age) = age_at(level, pos) else { return };
    let up = Face::Up.neighbour(pos);
    if age >= OLDEST - 1 || !is_source(level, up) {
        return;
    }
    let Some(id) = level.registry().find(KELP, &format!("{AGE}={}", age + 1)) else { return };
    send::set(world, layer, up, id);
}
