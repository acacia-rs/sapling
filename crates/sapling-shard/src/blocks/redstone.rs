//! Redstone: plates, buttons, levers, blocks of redstone and torches powering dust, lamps, doors,
//! trapdoors, fence gates and hoppers, by BDS's circuit system (docs/research/redstone.md). Every
//! second tick, at its end, the circuits written to are searched again from their sources
//! (search.rs); what changed is written into the blocks at the start of the next (apply.rs).
// TODO: circuits a world folder holds are found only once one of their blocks is written; no
// repeaters, comparators, pistons or observers; a torch burns out after 16 evaluations in a row
// that flipped it, where BDS counts those that its own power flipped, and relights by the next
// change beside it, where BDS first waits out 32.

mod apply;
mod parts;
mod search;

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::level;
use crate::components::Tick;
use parts::{Part, User};

/// Game ticks between evaluations (`Dimension::tickRedstone`).
const EVERY: u64 = 2;
const BURNOUT: u8 = 16;

/// What to write into a block at the start of the next tick.
#[derive(Clone, Copy, Debug)]
enum Due {
    Signal(u8),
    Torch { lit: bool, fizz: bool },
}

#[derive(Default)]
struct TorchState {
    input: u8,
    flips: u8,
    burnt: bool,
}

#[derive(Default)]
struct Circuit {
    sources: HashSet<BlockPos>,
    /// The signal each cell got at the last evaluation.
    power: HashMap<BlockPos, u8>,
    torches: HashMap<BlockPos, TorchState>,
    /// Cells written since the last evaluation.
    changed: HashSet<BlockPos>,
    /// Torches the last evaluation flipped: the next must look again.
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
    world
        .get_resource_or_insert_with(Circuits::default)
        .layers
        .entry(layer)
        .or_default()
        .changed
        .insert(pos);
}

/// The signal the block at `pos` got at the last evaluation.
pub(crate) fn power_at(world: &World, layer: Entity, pos: BlockPos) -> u8 {
    world
        .get_resource::<Circuits>()
        .and_then(|circuits| circuits.layers.get(&layer)?.power.get(&pos).copied())
        .unwrap_or(0)
}

/// Writes what the last evaluation changed, and puts out the lamps whose delay is up.
pub(crate) fn apply(world: &mut World) {
    let Some(mut circuits) = world.get_resource_mut::<Circuits>() else {
        return;
    };
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
    let Some(circuits) = world.get_resource_mut::<Circuits>() else {
        return;
    };
    let layers: Vec<Entity> = circuits
        .layers
        .iter()
        .filter(|(_, circuit)| !circuit.changed.is_empty() || circuit.unsettled)
        .map(|(layer, _)| *layer)
        .collect();
    for layer in layers {
        let Some(mut circuit) = world.resource_mut::<Circuits>().layers.remove(&layer) else {
            continue;
        };
        let due = evaluate_circuit(world, layer, &mut circuit);
        let mut circuits = world.resource_mut::<Circuits>();
        circuits.layers.insert(layer, circuit);
        circuits
            .due
            .extend(due.into_iter().map(|(pos, due)| (layer, pos, due)));
    }
}

/// What each cell of `layer` is to a circuit; `Other` where it is not loaded.
fn reader(world: &World, layer: Entity) -> impl FnMut(BlockPos) -> Part + '_ {
    let level = level(world, layer);
    move |[x, y, z]| match level {
        Some(level) if level.in_height(y) && level.is_loaded(super::send::column([x, y, z])) => {
            Part::of(level.registry(), level.block(x, y, z))
        }
        _ => Part::Other,
    }
}

fn evaluate_circuit(world: &World, layer: Entity, circuit: &mut Circuit) -> Vec<(BlockPos, Due)> {
    let mut part = reader(world, layer);
    let changed = std::mem::take(&mut circuit.changed);
    let mut fresh = HashSet::new();
    for pos in &changed {
        let now = part(*pos);
        if now.is_source() {
            circuit.sources.insert(*pos);
        }
        if now.is_written() {
            fresh.insert(*pos);
        }
        // A burnt torch relights after a change beside it.
        for face in Face::ALL {
            if let Some(torch) = circuit.torches.get_mut(&face.neighbour(*pos)) {
                torch.burnt = false;
            }
        }
    }
    circuit.sources.retain(|pos| part(*pos).is_source());
    circuit
        .torches
        .retain(|pos, _| circuit.sources.contains(pos));

    let mut due = Vec::new();
    let mut sources = Vec::new();
    for pos in &circuit.sources {
        let mut source = part(*pos);
        if let Part::Torch { lit, support } = source {
            let torch = circuit.torches.entry(*pos).or_default();
            let next = !torch.burnt && torch.input == 0;
            torch.flips = if next != lit {
                torch.flips.saturating_add(1)
            } else {
                0
            };
            let fizz = torch.flips >= BURNOUT;
            torch.burnt |= fizz;
            let next = next && !fizz;
            if next != lit {
                due.push((*pos, Due::Torch { lit: next, fizz }));
            }
            source = Part::Torch { lit: next, support };
        }
        sources.push((*pos, source));
    }
    circuit.unsettled = !due.is_empty();

    let power = search::search(&sources, &mut part);
    for (pos, torch) in &mut circuit.torches {
        torch.input = power.torches.get(pos).map_or(0, |(input, _)| *input);
    }
    let cells: HashSet<BlockPos> = power
        .at
        .keys()
        .chain(circuit.power.keys())
        .chain(&fresh)
        .copied()
        .collect();
    for pos in cells {
        let (old, new) = (
            circuit.power.get(&pos).copied().unwrap_or(0),
            power.at.get(&pos).copied().unwrap_or(0),
        );
        let is_fresh = fresh.contains(&pos);
        let write = match part(pos) {
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
    }
    circuit.power = power.at;
    due
}
