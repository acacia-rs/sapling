//! Blocks that hatch mobs (docs/research/mob-life.md): BDS's `TurtleEggBlock`, `FrogSpawnBlock` and
//! `SnifferEggBlock` subscribe to the place and queued-tick events. Writing one queues its first
//! tick ([`Job::Hatch`]); each tick then cracks it, hatches it or queues the next (turtle.rs).

mod frogspawn;
mod nests;
mod sniffer;
mod turtle;

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::queued::{self, Job};
use super::{Dice, level, send};

pub(crate) use nests::{dig as turtle_dig, egg_near as turtle_egg_near, lay as lay_turtle_eggs, nest_near as turtle_nest_near, stomp_out as stomp_turtle_eggs, stomped_at as stomping_turtle_eggs};
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
/// Cracks the egg at `pos` once more, heard as `sound`; one that `sparkles` names its cracked state
/// in a sparkle sent first (research/captures/bds-sniffers.txt).
fn crack(world: &mut World, layer: Entity, pos: BlockPos, sound: &str, sparkles: bool) {
    let Some(level) = level(world, layer) else { return };
    let id = level.block(pos[0], pos[1], pos[2]);
    let Some(cracked) = CRACKS.get(cracks(level, pos) + 1).and_then(|next| super::with_property(level.registry(), id, "cracked_state", next)) else { return };
    let heard = egg_sound(sound, pos);
    if sparkles {
        let sparkle = level_event(LevelEventEvent::ParticleTurtleEgg, pos, sapling_world::block_hash(level.registry(), cracked) as i32);
        send::to_viewers(world, layer, pos, &[sparkle]);
    }
    if send::set(world, layer, pos, cracked) {
        send::to_viewers(world, layer, pos, &[heard]);
    }
}

fn roll(world: &mut World, sides: u32) -> u32 {
    world.get_resource_or_insert_with(Dice::default).roll(sides)
}

fn corner([x, y, z]: BlockPos) -> Vec3f {
    Vec3f { x: x as f32, y: y as f32, z: z as f32 }
}

fn level_event(event: LevelEventEvent, pos: BlockPos, data: i32) -> bytes::Bytes {
    sapling_net::encode(&LevelEvent { event, position: corner(pos), data })
}

/// An egg's sounds come from the cell's corner and say nothing of the block (research/captures/bds-eggs.txt).
fn egg_sound(name: &str, pos: BlockPos) -> bytes::Bytes {
    send::noise(name, corner(pos), -1)
}

/// An egg's sparkle, naming the egg block there as BDS's does.
fn sparkle(level: &Level, pos: BlockPos) -> bytes::Bytes {
    let id = level.block(pos[0], pos[1], pos[2]);
    level_event(LevelEventEvent::ParticleTurtleEgg, pos, sapling_world::block_hash(level.registry(), id) as i32)
}

/// The destroy particles of a hatching egg, at the cell's corner as its sound.
fn hatch_burst(level: &Level, pos: BlockPos, id: u32) -> bytes::Bytes {
    level_event(LevelEventEvent::ParticleDestroy, pos, sapling_world::block_hash(level.registry(), id) as i32)
}

/// Whether the block `name` is one that hatches.
pub(super) fn is_egg(name: &str) -> bool {
    [turtle::EGG, frogspawn::SPAWN, sniffer::EGG].contains(&name)
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
        sniffer::EGG => sniffer::placed(world, layer, pos),
        _ => {}
    }
}

/// The queued tick of the egg at `pos`.
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos) {
    match level(world, layer).and_then(|level| name_at(level, pos)) {
        Some(turtle::EGG) => turtle::ticked(world, layer, pos),
        Some(frogspawn::SPAWN) => frogspawn::ticked(world, layer, pos),
        Some(sniffer::EGG) => sniffer::ticked(world, layer, pos),
        _ => {}
    }
}

/// Queues the egg at `pos`'s next tick `delay` ticks from now.
fn again(world: &mut World, layer: Entity, pos: BlockPos, delay: u64) {
    queued::after(world, layer, pos, delay, Job::Hatch);
}
