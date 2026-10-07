//! Sand, gravel and the like over a gap: the block becomes a falling entity that drops under
//! gravity (flight.rs) and is placed where it lands, or leaves its item where it cannot be, by
//! BDS's `FallingBlock` (docs/research/falling-blocks.md, and research/captures/ for what BDS sent).
// TODO: what is free to fall into and what breaks a falling block are name lists, not block
// data; concrete powder does not harden; no fall damage, splash or MOVING flag; the fall starts
// and lands with UpdateBlock where BDS sends UpdateBlockSynced (acacia-proto reads its entity id
// with the wrong varint).

mod flight;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::BlockRegistry;

use super::place::gives_way;
use super::{level, send};
use crate::components::Tick;

/// Snow layers fall on Bedrock (`TopSnowBlock` ticks as a falling block), where Java breaks them.
const FALLS: &[&str] = &[":sand", "red_sand", "suspicious_sand", "suspicious_gravel", "gravel", "concrete_powder", "dragon_egg", "anvil", "snow_layer"];
/// A falling block a stack's lowest is found through: all but snow (`canSlide`).
const SNOW: &str = "minecraft:snow_layer";

/// What changed for a falling block, which decides how soon it falls (BDS `onPlace`,
/// `neighborChanged`; a block placed by hand fell in the tick it was placed).
#[derive(Clone, Copy)]
pub(super) enum Change {
    Placed,
    Below,
    Beside,
}

impl Change {
    fn delay(self) -> u64 {
        match self {
            Change::Placed => 0,
            Change::Below => 1,
            Change::Beside => 2,
        }
    }
}

pub(super) fn falls(name: &str) -> bool {
    FALLS.iter().any(|end| name.ends_with(end))
}

/// Falling blocks whose fall is due.
#[derive(Resource, Default)]
pub(crate) struct Slides(Vec<(u64, Entity, BlockPos)>);

fn block(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(&BlockRegistry, u32)> {
    let level = level(world, layer)?;
    (level.in_height(y) && level.is_loaded(send::column([x, y, z]))).then(|| (level.registry(), level.block(x, y, z)))
}

fn name_at(world: &World, layer: Entity, pos: BlockPos) -> Option<&'static str> {
    block(world, layer, pos).and_then(|(registry, id)| registry.get(id)).map(|state| state.name)
}

/// After `change`: a falling block at `pos` is queued to fall.
pub(super) fn unsettled(world: &mut World, layer: Entity, pos: BlockPos, change: Change) {
    if !name_at(world, layer, pos).is_some_and(falls) {
        return;
    }
    let due = world.resource::<Tick>().0 + change.delay();
    let mut slides = world.get_resource_or_insert_with(Slides::default);
    if !slides.0.iter().any(|(_, at_layer, at)| (*at_layer, *at) == (layer, pos)) {
        slides.0.push((due, layer, pos));
    }
}

/// Whether a falling block over `below` drops into it (BDS `isFreeToFall`).
fn is_free(world: &World, layer: Entity, below: BlockPos) -> bool {
    block(world, layer, below).is_some_and(|(registry, id)| {
        gives_way(registry, id) || registry.get(id).is_some_and(|state| state.name == SNOW || state.name.contains("coral"))
    })
}

/// Sets the lowest falling block of the stack at `pos` falling, if it can (BDS `checkSlide`).
/// The cell it left, for its neighbours to settle.
fn slide(world: &mut World, layer: Entity, pos: BlockPos) -> Option<BlockPos> {
    let mut lowest = pos;
    while name_at(world, layer, Face::Down.neighbour(lowest)).is_some_and(|name| falls(name) && name != SNOW) {
        lowest = Face::Down.neighbour(lowest);
    }
    if !name_at(world, layer, pos).is_some_and(falls) || !is_free(world, layer, Face::Down.neighbour(lowest)) {
        return None;
    }
    let (registry, id) = block(world, layer, lowest)?;
    let air = registry.air_id();
    if !send::set(world, layer, lowest, air) {
        return None;
    }
    flight::start(world, layer, id, lowest)?;
    Some(lowest)
}

/// Moves the falling blocks and starts the falls that are due; the cells that changed, for their
/// neighbours to settle.
/// A fall starts before the tick's moves: on BDS a new falling block is a second tick on at once.
pub(super) fn tick(world: &mut World) -> Vec<(Entity, BlockPos)> {
    let now = world.resource::<Tick>().0;
    let mut changed = Vec::new();
    if let Some(mut slides) = world.get_resource_mut::<Slides>() {
        let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut slides.0).into_iter().partition(|(when, ..)| *when <= now);
        slides.0 = waiting;
        changed.extend(due.into_iter().filter_map(|(_, layer, pos)| Some((layer, slide(world, layer, pos)?))));
    }
    changed.extend(flight::fly(world));
    changed
}
