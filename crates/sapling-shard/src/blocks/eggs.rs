//! Blocks that hatch mobs (docs/research/mob-life.md): BDS's `TurtleEggBlock`, `FrogSpawnBlock` and
//! `SnifferEggBlock` subscribe to the place and queued-tick events. Writing one queues its first
//! tick ([`Job::Hatch`]); each tick then cracks it, hatches it or queues the next (turtle.rs).

mod frogspawn;
mod turtle;

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::queued::{self, Job};
use super::{Dice, level, send};

pub(crate) use turtle::trodden as tread_turtle_eggs;

type Level = sapling_world::World;

const CRACKS: [&str; 3] = ["no_cracks", "cracked", "max_cracked"];

fn name_at(level: &Level, [x, y, z]: BlockPos) -> Option<&'static str> {
    (level.in_height(y) && level.is_loaded(send::column([x, y, z]))).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name)
}

fn value_of<'a>(level: &'a Level, [x, y, z]: BlockPos, key: &str) -> Option<&'a str> {
    let state = level.registry().get(level.block(x, y, z))?;
    state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).map(|(_, value)| value)
}

/// How far the egg at `pos` has cracked, 0 to 2.
fn cracks(level: &Level, pos: BlockPos) -> usize {
    value_of(level, pos, "cracked_state").and_then(|state| CRACKS.iter().position(|crack| *crack == state)).unwrap_or(0)
}

/// Cracks the egg at `pos` once more, heard as `sound`.
fn crack(world: &mut World, layer: Entity, pos: BlockPos, sound: &str) {
    let Some(level) = level(world, layer) else { return };
    let id = level.block(pos[0], pos[1], pos[2]);
    let Some(cracked) = CRACKS.get(cracks(level, pos) + 1).and_then(|next| super::with_property(level.registry(), id, "cracked_state", next)) else { return };
    let heard = send::sound(sound, level.registry(), pos, id);
    if send::set(world, layer, pos, cracked) {
        send::to_viewers(world, layer, pos, &[heard]);
    }
}

fn roll(world: &mut World, sides: u32) -> u32 {
    world.get_resource_or_insert_with(Dice::default).roll(sides)
}

fn level_event(event: LevelEventEvent, [x, y, z]: BlockPos, data: i32) -> bytes::Bytes {
    sapling_net::encode(&LevelEvent { event, position: Vec3f { x: x as f32, y: y as f32, z: z as f32 }, data })
}

/// Whether the block `name` is one that hatches.
pub(super) fn is_egg(name: &str) -> bool {
    name == turtle::EGG || name == frogspawn::SPAWN
}

/// A block was written at `pos`, `was` before and `now` after: an egg that was not there queues
/// its first tick (the place event, which a change of state does not raise).
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos, was: &str, now: &str) {
    if was == now {
        return;
    }
    match now {
        turtle::EGG => turtle::placed(world, layer, pos),
        frogspawn::SPAWN => frogspawn::placed(world, layer, pos),
        _ => {}
    }
}

/// The queued tick of the egg at `pos`.
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos) {
    match level(world, layer).and_then(|level| name_at(level, pos)) {
        Some(turtle::EGG) => turtle::ticked(world, layer, pos),
        Some(frogspawn::SPAWN) => frogspawn::ticked(world, layer, pos),
        _ => {}
    }
}

/// Queues the egg at `pos`'s next tick `delay` ticks from now.
fn again(world: &mut World, layer: Entity, pos: BlockPos, delay: u64) {
    queued::after(world, layer, pos, delay, Job::Hatch);
}
