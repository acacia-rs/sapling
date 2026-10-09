//! Redstone: plates, buttons, levers, blocks of redstone, torches and repeaters powering dust, lamps,
//! doors, trapdoors, fence gates and hoppers, by BDS's circuit system (docs/research/redstone.md). Every
//! second tick, at its end, the circuits written to are searched again from their sources
//! (search.rs); what changed is written into the blocks at the start of the next (apply.rs).
// TODO: the circuits of an arena's template are found only once one of their blocks is written; no
// comparators, pistons or observers; a burnt torch whose loop was broken while it was out still
// waits for a change beside it.

mod apply;
mod capacitors;
mod parts;
mod search;

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::ChunkPos;

use super::level;
use crate::components::Tick;
use capacitors::{RepeaterState, TorchState};
use parts::{Part, User};

/// Game ticks between evaluations (`Dimension::tickRedstone`).
const EVERY: u64 = 2;
/// What to write into a block at the start of the next tick.
#[derive(Clone, Copy, Debug)]
enum Due {
    Signal(u8),
    /// A torch or repeater lit or not.
    Lit { lit: bool, fizz: bool },
}

#[derive(Default)]
struct Circuit {
    sources: HashSet<BlockPos>,
    /// The signal each cell got at the last evaluation.
    power: HashMap<BlockPos, u8>,
    torches: HashMap<BlockPos, TorchState>,
    repeaters: HashMap<BlockPos, RepeaterState>,
    /// Cells written since the last evaluation.
    changed: HashSet<BlockPos>,
    /// The last evaluation flipped a torch or repeater, or one waits to flip: the next must look again.
    unsettled: bool,
}

#[derive(Resource, Default)]
pub(crate) struct Circuits {
    layers: HashMap<Entity, Circuit>,
    due: Vec<(Entity, BlockPos, Due)>,
    lamps_off: Vec<(u64, Entity, BlockPos)>,
}

/// Notes a cell written, for the next evaluation to look at.
pub(super) fn notice(world: &mut World, layer: Entity, pos: BlockPos) {
    world.get_resource_or_insert_with(Circuits::default).layers.entry(layer).or_default().changed.insert(pos);
}

/// Notes the sources and dust of a column that came from storage: a circuit saved with the world
/// is searched at the next evaluation and works on from there.
pub(crate) fn arrived(world: &mut World, layer: Entity, column: ChunkPos) {
    let Some(level) = level(world, layer) else { return };
    let registry = level.registry();
    let found = level.find_blocks(column, |id| matches!(Part::of(registry, id), part if part.is_source() || matches!(part, Part::Dust { .. })));
    if !found.is_empty() {
        world.get_resource_or_insert_with(Circuits::default).layers.entry(layer).or_default().changed.extend(found);
    }
}

/// The signal the block at `pos` got at the last evaluation.
pub(crate) fn power_at(world: &World, layer: Entity, pos: BlockPos) -> u8 {
    world.get_resource::<Circuits>().and_then(|circuits| circuits.layers.get(&layer)?.power.get(&pos).copied()).unwrap_or(0)
}

/// Writes what the last evaluation changed, and puts out the lamps whose delay is up.
pub(crate) fn apply(world: &mut World) {
    let Some(mut circuits) = world.get_resource_mut::<Circuits>() else { return };
    let due = std::mem::take(&mut circuits.due);
    for (layer, pos, due) in due {
        apply::write(world, layer, pos, due);
    }
    apply::put_out_lamps(world);
}

/// Searches every circuit written to since the last evaluation, on the ticks BDS evaluates.
pub(crate) fn evaluate(world: &mut World) {
    if world.resource::<Tick>().0 % EVERY != 0 {
        return;
    }
    let Some(circuits) = world.get_resource_mut::<Circuits>() else { return };
    let layers: Vec<Entity> = circuits.layers.iter().filter(|(_, circuit)| !circuit.changed.is_empty() || circuit.unsettled).map(|(layer, _)| *layer).collect();
    for layer in layers {
        let Some(mut circuit) = world.resource_mut::<Circuits>().layers.remove(&layer) else { continue };
        let now = world.resource::<Tick>().0;
        let due = evaluate_circuit(world, layer, &mut circuit, now);
        let mut circuits = world.resource_mut::<Circuits>();
        circuits.layers.insert(layer, circuit);
        circuits.due.extend(due.into_iter().map(|(pos, due)| (layer, pos, due)));
    }
}

/// What each cell of `layer` is to a circuit; `Other` where it is not loaded.
fn reader(world: &World, layer: Entity) -> impl FnMut(BlockPos) -> Part + '_ {
    let level = level(world, layer);
    move |[x, y, z]| match level {
        Some(level) if level.in_height(y) && level.is_loaded(super::send::column([x, y, z])) => Part::of(level.registry(), level.block(x, y, z)),
        _ => Part::Other,
    }
}

/// Searches from the sources as they are, which is what dust shows and what torches and repeaters
/// flip by; what they give once flipped reaches lamps, doors and hoppers in the same evaluation,
/// by a second search (research/captures/bds-redstone-repeaters.txt).
fn evaluate_circuit(world: &World, layer: Entity, circuit: &mut Circuit, now: u64) -> Vec<(BlockPos, Due)> {
    let mut part = reader(world, layer);
    let changed = std::mem::take(&mut circuit.changed);
    let mut fresh = HashSet::new();
    for pos in &changed {
        let put = part(*pos);
        if put.is_source() {
            circuit.sources.insert(*pos);
        }
        if put.is_written() {
            fresh.insert(*pos);
        }
        for face in Face::ALL {
            if let Some(torch) = circuit.torches.get_mut(&face.neighbour(*pos)) {
                torch.reignite(now);
            }
        }
    }
    circuit.sources.retain(|pos| part(*pos).is_source());
    circuit.torches.retain(|pos, _| circuit.sources.contains(pos));
    circuit.repeaters.retain(|pos, _| circuit.sources.contains(pos));

    let mut sources: Vec<(BlockPos, Part)> = circuit.sources.iter().map(|pos| (*pos, part(*pos))).collect();
    let shown = search::search(&sources, &mut part);
    let mut due = Vec::new();
    for (pos, source) in &mut sources {
        let (lit, next, fizz) = match *source {
            Part::Torch { lit, .. } => {
                let (next, fizz) = circuit.torches.entry(*pos).or_default().next(*pos, lit, shown.inputs.get(pos).copied(), now);
                (lit, next, fizz)
            }
            Part::Repeater { lit, delay, .. } => {
                let fed = shown.inputs.get(pos).is_some_and(|(input, _)| *input > 0);
                (lit, circuit.repeaters.entry(*pos).or_default().next(lit, fed, delay, shown.locked.contains(pos)), false)
            }
            _ => continue,
        };
        if next != lit {
            due.push((*pos, Due::Lit { lit: next, fizz }));
            *source = match *source {
                Part::Torch { support, .. } => Part::Torch { lit: next, support },
                Part::Repeater { input, delay, .. } => Part::Repeater { lit: next, input, delay },
                other => other,
            };
        }
    }
    circuit.unsettled = !due.is_empty() || circuit.repeaters.values().any(RepeaterState::is_waiting);
    let flipped = (!due.is_empty()).then(|| search::search(&sources, &mut part));
    let used = flipped.as_ref().unwrap_or(&shown);

    let cells: HashSet<BlockPos> = shown.at.keys().chain(used.at.keys()).chain(circuit.power.keys()).chain(&fresh).copied().collect();
    let mut power = HashMap::new();
    for pos in cells {
        let cell = part(pos);
        let found = if matches!(cell, Part::User(_)) { used } else { &shown };
        let (old, new) = (circuit.power.get(&pos).copied().unwrap_or(0), found.at.get(&pos).copied().unwrap_or(0));
        let is_fresh = fresh.contains(&pos);
        let write = match cell {
            Part::Dust { signal } => signal != new,
            // Doors, trapdoors and gates skip the update their first evaluation brings (R8).
            Part::User(User::Opens(_)) if is_fresh => false,
            // A user put unpowered keeps the state it was put in, as a /setblock-pressed button does.
            Part::User(_) => new != old || (is_fresh && new > 0),
            _ => false,
        };
        if write {
            due.push((pos, Due::Signal(new)));
        }
        if new > 0 {
            power.insert(pos, new);
        }
    }
    circuit.power = power;
    due
}
