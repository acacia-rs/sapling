//! Redstone: plates, buttons, levers, blocks of redstone, torches, repeaters, comparators,
//! observers, tripwire hooks, targets, sculk sensors, lightning rods and jukeboxes powering dust,
//! lamps, copper bulbs, pistons, doors, trapdoors, fence gates, hoppers, note blocks, dispensers,
//! droppers and crafters, by BDS's
//! circuit system (docs/research/redstone.md). Every second tick, at its start, the circuits
//! written to are searched again from their sources (search.rs); what changed is written into
//! the blocks at the start of the next (apply.rs).
// TODO: the circuits of an arena's template are found only once one of their blocks is written;
// a comparator reads no lectern; a burnt torch whose loop was broken while it was out still
// waits for a change beside it.

mod apply;
mod capacitors;
mod observers;
mod parts;
mod rail_line;
mod reads;
mod search;

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::ChunkPos;

use super::level;
use crate::components::Tick;
use capacitors::{RepeaterState, TorchState, compared};
use parts::{Part, User};
use reads::read_containers;

/// Game ticks between evaluations (`Dimension::tickRedstone`).
const EVERY: u64 = 2;
/// What to write into a block at the start of the next tick.
#[derive(Clone, Copy, Debug)]
enum Due {
    /// The signal now and at the evaluation before.
    Signal(u8, u8),
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
    /// What each comparator gives out: its block says only whether it is lit.
    comparators: HashMap<BlockPos, u8>,
    /// Whether each observer was lit at the last evaluation: dust shows that, an evaluation late.
    observers: HashMap<BlockPos, bool>,
    /// The strength of each producer whose block does not store it ([`emit`]).
    held: HashMap<BlockPos, u8>,
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
    observers: observers::Watch,
}

/// Notes a cell written, for the next evaluation to look at.
pub(super) fn notice(world: &mut World, layer: Entity, pos: BlockPos) {
    let tick = world.get_resource::<Tick>().map_or(0, |tick| tick.0);
    let mut circuits = world.get_resource_or_insert_with(Circuits::default);
    circuits.layers.entry(layer).or_default().changed.insert(pos);
    circuits.observers.written.push((tick, layer, pos));
}

/// Notes the sources and dust of a column that came from storage: a circuit saved with the world
/// is searched at the next evaluation and works on from there.
pub(crate) fn arrived(world: &mut World, layer: Entity, column: ChunkPos) {
    let Some(level) = level(world, layer) else { return };
    let registry = level.registry();
    let found = level.find_blocks(column, |id| matches!(Part::of(registry, id), part if part.is_source() || matches!(part, Part::Dust { .. })));
    if found.is_empty() {
        return;
    }
    // A jukebox keeps its disc in its entity, which the block's state does not tell.
    let playing: Vec<(BlockPos, u8)> = found.iter().map(|pos| (*pos, super::jukebox::power(world, layer, *pos))).filter(|(_, power)| *power > 0).collect();
    super::sculk::found(world, layer, &found);
    let mut circuits = world.get_resource_or_insert_with(Circuits::default);
    let circuit = circuits.layers.entry(layer).or_default();
    circuit.held.extend(playing);
    circuit.changed.extend(found);
}

/// Notes a container whose slots changed, where a comparator may read it.
pub(crate) fn container_changed(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(mut circuits) = world.get_resource_mut::<Circuits>() else { return };
    if let Some(circuit) = circuits.layers.get_mut(&layer).filter(|circuit| !circuit.comparators.is_empty()) {
        circuit.changed.insert(pos);
    }
}

/// Holds the strength of the producer at `pos` whose block does not store it (a target, a sculk
/// sensor, a jukebox), to be searched from at the next evaluation; 0 lets it go.
pub(crate) fn emit(world: &mut World, layer: Entity, pos: BlockPos, strength: u8) {
    let mut circuits = world.get_resource_or_insert_with(Circuits::default);
    let circuit = circuits.layers.entry(layer).or_default();
    if strength > 0 {
        circuit.held.insert(pos, strength);
    } else {
        circuit.held.remove(&pos);
    }
    circuit.changed.insert(pos);
}

/// The signal the block at `pos` got at the last evaluation.
pub(crate) fn power_at(world: &World, layer: Entity, pos: BlockPos) -> u8 {
    world.get_resource::<Circuits>().and_then(|circuits| circuits.layers.get(&layer)?.power.get(&pos).copied()).unwrap_or(0)
}

/// Writes what the last evaluation changed, puts out the lamps whose delay is up, and turns the
/// observers.
pub(crate) fn apply(world: &mut World) {
    let Some(mut circuits) = world.get_resource_mut::<Circuits>() else { return };
    let due = std::mem::take(&mut circuits.due);
    for (layer, pos, due) in due {
        apply::write(world, layer, pos, due);
    }
    apply::put_out_lamps(world);
    observers::tick(world);
}

/// Searches every circuit written to since the last evaluation, on the ticks BDS evaluates.
pub(crate) fn evaluate(world: &mut World) {
    if world.resource::<Tick>().0 % EVERY != 0 {
        return;
    }
    let Some(circuits) = world.get_resource_mut::<Circuits>() else { return };
    let mut layers: Vec<Entity> = circuits.layers.iter().filter(|(_, circuit)| !circuit.changed.is_empty() || circuit.unsettled).map(|(layer, _)| *layer).collect();
    // `sapling:redstone` off pauses a world's circuits: what was written waits for it to be on.
    layers.retain(|layer| crate::rules::on(world, *layer, sapling_data::REDSTONE));
    for layer in layers {
        let Some(mut circuit) = world.resource_mut::<Circuits>().layers.remove(&layer) else { continue };
        let now = world.resource::<Tick>().0;
        let stored = read_containers(world, layer, &circuit);
        let due = evaluate_circuit(world, layer, &mut circuit, now, &stored);
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
fn evaluate_circuit(world: &World, layer: Entity, circuit: &mut Circuit, now: u64, stored: &HashMap<BlockPos, u8>) -> Vec<(BlockPos, Due)> {
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
    circuit.comparators.retain(|pos, _| circuit.sources.contains(pos));
    circuit.observers.retain(|pos, _| circuit.sources.contains(pos));
    circuit.held.retain(|pos, _| circuit.sources.contains(pos));

    let as_kept = |pos: &BlockPos, part: Part| match part {
        Part::Producer { strength, strong } => Part::Producer { strength: strength.max(circuit.held.get(pos).copied().unwrap_or(0)), strong },
        Part::Comparator { lit, input, subtracts, output } => Part::Comparator { lit, input, subtracts, output: circuit.comparators.get(pos).copied().unwrap_or(output) },
        Part::Observer { lit, watch } => Part::Observer { lit: circuit.observers.get(pos).copied().unwrap_or(lit), watch },
        other => other,
    };
    let mut sources: Vec<(BlockPos, Part)> = circuit.sources.iter().map(|pos| (*pos, as_kept(pos, part(*pos)))).collect();
    let mut restrengthened = false;
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
            Part::Observer { lit, watch } => {
                // Its block is turned by observers.rs: only dust is behind.
                let Part::Observer { lit: now_lit, .. } = part(*pos) else { continue };
                restrengthened |= now_lit != lit;
                circuit.observers.insert(*pos, now_lit);
                *source = Part::Observer { lit: now_lit, watch };
                continue;
            }
            Part::Comparator { lit, input, subtracts, output } => {
                let behind = shown.inputs.get(pos).map_or(0, |(input, _)| *input).max(stored.get(pos).copied().unwrap_or(0));
                let next = compared(behind, shown.sides.get(pos).copied().unwrap_or(0), subtracts);
                circuit.comparators.insert(*pos, next);
                restrengthened |= next != output;
                *source = Part::Comparator { lit, input, subtracts, output: next };
                (lit, next > 0, false)
            }
            _ => continue,
        };
        if next != lit {
            due.push((*pos, Due::Lit { lit: next, fizz }));
            *source = match *source {
                Part::Torch { support, .. } => Part::Torch { lit: next, support },
                Part::Repeater { input, delay, .. } => Part::Repeater { lit: next, input, delay },
                Part::Comparator { input, subtracts, output, .. } => Part::Comparator { lit: next, input, subtracts, output },
                other => other,
            };
        }
    }
    circuit.unsettled = !due.is_empty() || restrengthened || circuit.repeaters.values().any(RepeaterState::is_waiting);
    let flipped = (!due.is_empty() || restrengthened).then(|| search::search(&sources, &mut part));
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
            due.push((pos, Due::Signal(new, old)));
        }
        if new > 0 {
            power.insert(pos, new);
        }
    }
    circuit.power = power;
    due
}
