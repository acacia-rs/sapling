//! Pointed dripstone hanging from nothing: the whole stalactite falls at once, tip first, and each
//! piece breaks where it lands, leaving its item; the tip hurts what it lands on
//! (research/captures/bds-dripstone.txt).

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{BlockPos, Face, Item};
use sapling_world::BlockRegistry;

use super::super::{level, loot, send};
use super::{block, flight};
use crate::body::Body;
use crate::components::{InLayer, Position, in_world};
use crate::creatures::{self, Cause, Creature};
use crate::vitals;

const NAME: &str = "minecraft:pointed_dripstone";
const MOST_HARM_PER_BLOCK: usize = 6;
const MAX_HARM: f32 = 40.0;
/// A player's box, for what a tip lands on.
const PLAYER_SIZE: [f32; 2] = [0.6, 1.8];

/// Whether the block `id` is pointed dripstone hanging down.
pub(super) fn hangs(registry: &BlockRegistry, id: u32) -> bool {
    registry.get(id).is_some_and(|state| state.name == NAME && state.properties.contains("hanging=1"))
}

pub(super) fn is_dripstone(registry: &BlockRegistry, id: u32) -> bool {
    registry.get(id).is_some_and(|state| state.name == NAME)
}

fn is_tip(registry: &BlockRegistry, id: u32) -> bool {
    registry.get(id).is_some_and(|state| state.properties.contains("dripstone_thickness=tip"))
}

/// Whether the stalactite piece at `pos` holds: what is over it is solid or more of it.
fn holds(world: &World, layer: Entity, pos: BlockPos) -> bool {
    block(world, layer, Face::Up.neighbour(pos)).is_some_and(|(registry, id)| hangs(registry, id) || registry.get(id).is_some_and(|state| state.is_solid()))
}

/// Sets the stalactite from `pos` down falling where nothing holds it: its upper pieces are emptied
/// together, then the tip leaves as a falling block would, and they fall tip first. The cell that
/// changed, for its neighbours.
pub(super) fn fall(world: &mut World, layer: Entity, pos: BlockPos) -> Option<BlockPos> {
    if holds(world, layer, pos) {
        return None;
    }
    let mut pieces = Vec::new();
    let mut at = pos;
    while let Some((registry, id)) = block(world, layer, at).filter(|(registry, id)| hangs(registry, *id)) {
        pieces.push((at, id));
        if is_tip(registry, id) {
            break;
        }
        at = Face::Down.neighbour(at);
    }
    let (&(tip, _), upper) = pieces.split_last()?;
    let air = level(world, layer)?.registry().air_id();
    send::set_many(world, layer, &upper.iter().map(|&(at, _)| (at, air)).collect::<Vec<_>>());
    let falling = super::falling_from(world, layer, tip)?;
    // Only the tip hurts, by the pieces over it (BDS `PointedDripstoneBlock::startFalling` 14660e100).
    flight::hurts(world, falling, (1 + upper.len()).min(MOST_HARM_PER_BLOCK) as f32);
    for &(at, id) in upper.iter().rev() {
        flight::start(world, layer, id, at);
    }
    Some(pos)
}

/// A piece landing with its box at `body` after falling `fell` hurts the mobs and players its box
/// reaches, `per_block` for each block past the first, up to 40 (BDS
/// `FallingBlockActor::causeFallDamageToActor` 145985ca0).
pub(super) fn hurt(world: &mut World, layer: Entity, body: &Body, per_block: f32, fell: f32) {
    let blocks = (fell - 1.0).ceil();
    if per_block <= 0.0 || blocks <= 0.0 {
        return;
    }
    let cost = (blocks * per_block).floor().min(MAX_HARM);
    let reaches = |other: &Body| {
        let ((low, high), (other_low, other_high)) = (body.bounds(), other.bounds());
        (0..3).all(|axis| low[axis] < other_high[axis] && other_low[axis] < high[axis])
    };
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body), With<Creature>>();
    let struck: Vec<Entity> = mobs.iter(world).filter(|(_, on, other)| on.0 == layer && reaches(other)).map(|(mob, ..)| mob).collect();
    for mob in struck {
        creatures::wound(world, mob, layer, cost, Cause::Stalactite);
    }
    let players: Vec<Entity> = in_world(world)
        .into_iter()
        .filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer))
        .filter(|player| world.get::<Position>(*player).is_some_and(|position| reaches(&Body::new(position.feet, [0.0; 3], PLAYER_SIZE))))
        .collect();
    for player in players {
        vitals::hurt(world, player, cost, Cause::Stalactite);
    }
}

fn event(event: LevelEventEvent, [x, y, z]: BlockPos, data: i32) -> Bytes {
    sapling_net::encode(&LevelEvent { event, position: Vec3f { x: x as f32, y: y as f32, z: z as f32 }, data })
}

/// What viewers are told as a piece lands in `cell`, before its landing move.
pub(super) fn landed(world: &World, layer: Entity, block: u32, cell: BlockPos) -> Vec<Bytes> {
    let Some(hash) = level(world, layer).map(|level| level.registry().network_hash(block) as i32) else { return Vec::new() };
    vec![event(LevelEventEvent::ParticleDestroyBlockNoSound, cell, hash), event(LevelEventEvent::PointedDripstoneLand, cell, hash)]
}

/// A landed piece is put in `cell` and breaks there at once, as BDS showed it: both in one
/// UpdateSubchunkBlocks, the cell left as it was, and its item let go.
pub(super) fn shatter(world: &mut World, layer: Entity, block: u32, cell: BlockPos) {
    let Some(here) = level(world, layer).filter(|level| level.in_height(cell[1])).map(|level| level.block(cell[0], cell[1], cell[2])) else { return };
    send::show_states(world, layer, &[(cell, block), (cell, here)]);
    loot::spill(world, layer, cell, &[Item::new(NAME, 1)]);
}
