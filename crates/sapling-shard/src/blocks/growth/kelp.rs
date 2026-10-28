//! Kelp, weeping vines and twisting vines grow on a tick they schedule themselves on BDS's
//! random-tick queue, not by a random tick: a wait drawn to match the Java game's odds a random
//! tick at a speed of 3 (`KelpBlock` 142fdc460), on a queue that runs `randomTickSpeed` steps a
//! tick. BDS's kelp at 200 put on a cell every 48 ticks or so (research/captures/bds-plants.txt),
//! the drawn wait's mean of 9750 steps over 200. The nether vines subscribe to the queued tick
//! and to no random tick, as kelp does (docs/research/world-ticks.md, "Nether vines").

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_data::RuleValue;

use super::super::{level, send};
use super::{Picks, RULE};

/// A plant that grows from its tip: its block, age property, the way it grows, whether it grows
/// into a water source (else air), the Java game's chance a random tick, and the age it stops at.
struct Plant {
    name: &'static str,
    age: &'static str,
    grows: Face,
    in_water: bool,
    odds: f64,
    oldest: u8,
}

const PLANTS: [Plant; 3] = [
    Plant { name: "minecraft:kelp", age: "kelp_age", grows: Face::Up, in_water: true, odds: 0.14, oldest: 24 },
    Plant { name: "minecraft:weeping_vines", age: "weeping_vines_age", grows: Face::Down, in_water: false, odds: 0.1, oldest: 25 },
    Plant { name: "minecraft:twisting_vines", age: "twisting_vines_age", grows: Face::Up, in_water: false, odds: 0.1, oldest: 25 },
];
const DRAWN_SPEED: f64 = 3.0;
const CELLS: f64 = 4096.0;

/// Each world's random-tick queue: its clock and the plants due on it.
#[derive(Resource, Default)]
pub(crate) struct Kelp {
    clock: HashMap<Entity, u64>,
    due: Vec<(u64, Entity, BlockPos)>,
    /// Plants written since the last tick, whose wait is drawn then.
    fresh: Vec<(Entity, BlockPos)>,
}

fn plant(name: &str) -> Option<&'static Plant> {
    PLANTS.iter().find(|plant| plant.name == name)
}

/// Whether `name` is a plant this module grows.
pub(super) fn is_grown(name: &str) -> bool {
    plant(name).is_some()
}

/// The plant at `pos` and its age.
fn aged_at(level: &sapling_world::World, [x, y, z]: BlockPos) -> Option<(&'static Plant, u8)> {
    let state = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten()?;
    let plant = plant(state.name)?;
    let age = state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(key, _)| *key == plant.age).and_then(|(_, value)| value.parse().ok())?;
    Some((plant, age))
}

fn takes(level: &sapling_world::World, plant: &Plant, [x, y, z]: BlockPos) -> bool {
    level.in_height(y)
        && level.is_loaded(send::column([x, y, z]))
        && level.registry().get(level.block(x, y, z)).is_some_and(|state| if plant.in_water { state.name == "minecraft:water" && state.liquid_depth == 0 } else { state.is_air() })
}

/// Steps of the queue to the next growth: a geometric wait (`floor(ln u / ln(1 - p)) + 1`).
fn wait(picks: &mut Picks, odds: f64) -> u64 {
    let draw = (f64::from(picks.below(1 << 24)) + 1.0) / f64::from(1u32 << 24);
    (draw.ln() / (1.0 - odds * DRAWN_SPEED / CELLS).ln()).floor() as u64 + 1
}

impl Kelp {
    /// Queues the plant at `pos` to grow after `steps` of its world's queue, unless it waits already.
    fn queue(&mut self, layer: Entity, pos: BlockPos, steps: u64) {
        if !self.due.iter().any(|(_, at_layer, at)| (*at_layer, *at) == (layer, pos)) {
            let now = self.clock.get(&layer).copied().unwrap_or(0);
            self.due.push((now + steps, layer, pos));
        }
    }
}

/// A plant written at `pos` waits to grow.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    if level(world, layer).and_then(|level| aged_at(level, pos)).is_some() {
        world.get_resource_or_insert_with(Kelp::default).fresh.push((layer, pos));
    }
}

/// The tips among `cells`, a column's plants that came in.
pub(super) fn arrived(world: &mut World, layer: Entity, cells: &[BlockPos]) {
    let Some(level) = level(world, layer) else { return };
    let is_tip = |pos: &BlockPos| aged_at(level, *pos).is_some_and(|(plant, _)| aged_at(level, plant.grows.neighbour(*pos)).is_none_or(|(next, _)| next.name != plant.name));
    let tips: Vec<(Entity, BlockPos)> = cells.iter().filter(|pos| is_tip(pos)).map(|pos| (layer, *pos)).collect();
    if !tips.is_empty() {
        world.get_resource_or_insert_with(Kelp::default).fresh.extend(tips);
    }
}

/// Bone meal on kelp: the top of its stalk, if it waits to grow, grows the next tick instead
/// (142fdcbe0); false where nothing waited.
pub(crate) fn meal(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(level) = level(world, layer) else { return false };
    let top = (0..).map(|up| [pos[0], pos[1] + up, pos[2]]).take_while(|at| aged_at(level, *at).is_some()).last().unwrap_or(pos);
    let Some(mut kelp) = world.get_resource_mut::<Kelp>() else { return false };
    let now = kelp.clock.get(&layer).copied().unwrap_or(0);
    let Some(entry) = kelp.due.iter_mut().find(|(_, at_layer, at)| (*at_layer, *at) == (layer, top)) else { return false };
    entry.0 = now;
    true
}

/// Runs each world's queue on by its `randomTickSpeed` and grows the plants due: a cell on into
/// what it takes, a year older, which waits in turn.
pub(super) fn tick(world: &mut World, picks: &mut Picks) {
    let Some(mut kelp) = world.remove_resource::<Kelp>() else { return };
    for (layer, pos) in std::mem::take(&mut kelp.fresh) {
        let Some((plant, _)) = level(world, layer).and_then(|level| aged_at(level, pos)) else { continue };
        let steps = wait(picks, plant.odds);
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

/// The new tip is written, which queues it in turn.
fn grow(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let Some((plant, age)) = aged_at(level, pos) else { return };
    let next = plant.grows.neighbour(pos);
    if age >= plant.oldest || !takes(level, plant, next) {
        return;
    }
    let Some(id) = level.registry().find(plant.name, &format!("{}={}", plant.age, age + 1)) else { return };
    send::set(world, layer, next, id);
}
