//! What a comparator reads off the block behind it, or behind the block behind it: a block by its
//! state, a jukebox by its disc, a pot, a crafter, a sculk sensor, or a container by how full it is.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::capacitors::{fullness, state_signal};
use super::parts::Part;
use super::{Circuit, reader};
use crate::blocks::level;
use crate::containers;

/// What each comparator of `circuit` reads.
pub(super) fn read_containers(world: &mut World, layer: Entity, circuit: &Circuit) -> HashMap<BlockPos, u8> {
    let mut part = reader(world, layer);
    let behind: Vec<(BlockPos, BlockPos, Option<BlockPos>, Face)> = (circuit.sources.iter().chain(&circuit.changed))
        .filter_map(|pos| match part(*pos) {
            Part::Comparator { input, .. } => {
                let near = input.neighbour(*pos);
                Some((*pos, near, (part(near) == Part::Conductor).then(|| input.neighbour(near)), input))
            }
            _ => None,
        })
        .collect();
    drop(part);
    behind.into_iter().filter_map(|(pos, near, far, input)| Some((pos, read(world, layer, near).or_else(|| read_past(world, layer, far?, input))?))).collect()
}

/// What a comparator reads past a solid block, at `far`: the block there, and a frame on the
/// block's back facing `input`, whichever is stronger.
fn read_past(world: &mut World, layer: Entity, far: BlockPos, input: Face) -> Option<u8> {
    let framed = crate::blocks::frames::signal(world, layer, far, input);
    read(world, layer, far).max(framed)
}

fn by_state(world: &World, layer: Entity, at: BlockPos) -> Option<u8> {
    let level = level(world, layer).filter(|level| level.in_height(at[1]))?;
    let state = level.registry().get(level.block(at[0], at[1], at[2]))?;
    state_signal(state.name, state.properties)
}

fn read(world: &mut World, layer: Entity, at: BlockPos) -> Option<u8> {
    let pot = |world: &World| crate::blocks::holders::potted(world, layer, at).map(|inside| fullness(inside.iter().map(|held| (held.count, sapling_data::max_stack(&held.name))), 1));
    by_state(world, layer, at)
        .or_else(|| crate::blocks::jukebox::signal(world, layer, at))
        .or_else(|| crate::blocks::lectern::signal(world, layer, at))
        .or_else(|| crate::blocks::command_blocks::signal(world, layer, at))
        .or_else(|| crate::blocks::bookshelves::signal(world, layer, at))
        .or_else(|| pot(world))
        .or_else(|| crate::blocks::sculk::signal(world, layer, at))
        .or_else(|| crate::blocks::crafter::signal(world, layer, at))
        .or_else(|| {
            containers::with_slots(world, layer, at, |_, slots| {
                let filled = slots.iter().flatten().map(|held| (held.stack.count, sapling_data::max_stack(&held.stack.name)));
                fullness(filled, slots.len())
            })
        })
}
