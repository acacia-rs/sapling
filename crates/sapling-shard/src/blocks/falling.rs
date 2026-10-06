//! Sand, gravel and the like over a gap: the block becomes a falling entity that drops under
//! gravity and is placed where it lands, or leaves its item where it cannot be, by BDS's
//! `FallingBlock` and `FallingBlockActor` (docs/research/falling-blocks.md).
// TODO: what is free to fall into and what breaks a falling block are name lists, not block
// data; concrete powder does not harden; no fall damage; the movement packets are a guess, as no
// BDS fall was recorded.

use acacia_proto::packets::{LevelEvent, LevelEventEvent, MoveEntityDelta, SetEntityMotion};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{BlockPos, Face, Item};
use sapling_world::BlockRegistry;

use super::place::gives_way;
use super::{level, loot, send};
use crate::components::{NetId, Tick};
use crate::mobs;

/// Snow layers fall on Bedrock (`TopSnowBlock` ticks as a falling block), where Java breaks them.
const FALLS: &[&str] = &[":sand", "red_sand", "suspicious_sand", "suspicious_gravel", "gravel", "concrete_powder", "dragon_egg", "anvil", "snow_layer"];
/// A falling block a stack's lowest is found through: all but snow (`canSlide`).
const SNOW: &str = "minecraft:snow_layer";
/// Ticks from a block being placed, or a neighbour other than the one below changing, to it
/// falling; one when the block below changed.
const DELAY: u64 = 2;
const GRAVITY: f32 = 0.04;
const DRAG: f32 = 0.98;
/// Ticks in the air before it gives up and leaves its item; below the world, before it vanishes.
const MAX_AIRBORNE: u32 = 600;
const MAX_BELOW_WORLD: u32 = 100;
/// It stays a few ticks after it landed, so that clients see the block before it goes.
const LINGER: u32 = 3;
/// How far above its bottom the entity's position is on the wire: BDS spawns it at the middle of
/// the cell it leaves.
const WIRE_OFFSET: f32 = 0.5;

pub(super) fn falls(name: &str) -> bool {
    FALLS.iter().any(|end| name.ends_with(end))
}

/// Falling blocks whose fall is due.
#[derive(Resource, Default)]
pub(crate) struct Slides(Vec<(u64, Entity, BlockPos)>);

#[derive(Component)]
pub(crate) struct FallingBlock {
    layer: Entity,
    block: u32,
    /// The middle of its bottom face.
    at: [f32; 3],
    fall_speed: f32,
    airborne: u32,
    landed_for: Option<u32>,
}

fn block(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(&BlockRegistry, u32)> {
    let level = level(world, layer)?;
    (level.in_height(y) && level.is_loaded(send::column([x, y, z]))).then(|| (level.registry(), level.block(x, y, z)))
}

fn name_at(world: &World, layer: Entity, pos: BlockPos) -> Option<&'static str> {
    block(world, layer, pos).and_then(|(registry, id)| registry.get(id)).map(|state| state.name)
}

/// After a change beside `pos`: a falling block there falls soon, sooner when the block under
/// it is what changed (BDS `FallingBlock::neighborChanged`).
pub(super) fn unsettled(world: &mut World, layer: Entity, pos: BlockPos, from_below: bool) {
    if !name_at(world, layer, pos).is_some_and(falls) {
        return;
    }
    let due = world.resource::<Tick>().0 + if from_below { 1 } else { DELAY };
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
    let (hash, air) = (registry.network_hash(id), registry.air_id());
    if !send::set(world, layer, lowest, air) {
        return None;
    }
    let at = [lowest[0] as f32 + 0.5, lowest[1] as f32, lowest[2] as f32 + 0.5];
    let falling = FallingBlock { layer, block: id, at, fall_speed: 0.0, airborne: 0, landed_for: None };
    let entity = mobs::spawn(world, layer, at, |net_id| Some(announcement(net_id, &falling, hash)))?;
    world.entity_mut(entity).insert(falling);
    Some(lowest)
}

fn wire([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, y + WIRE_OFFSET, z]
}

fn announcement(id: u64, falling: &FallingBlock, hash: u32) -> Vec<Bytes> {
    vec![sapling_net::encode(&sapling_data::add_falling_block(id, wire(falling.at), [0.0, -falling.fall_speed, 0.0], hash))]
}

/// The top of the highest collision box in the cells from `from` down to `to`, if one is in
/// that span: where a block falling from `from` to `to` comes to rest.
fn floor_between(world: &World, layer: Entity, [x, from, z]: [f32; 3], to: f32) -> Option<f32> {
    let level = level(world, layer)?;
    let cell = [x.floor() as i32, z.floor() as i32];
    (to.floor() as i32..=from.floor() as i32).rev().find_map(|y| {
        let state = level.in_height(y).then(|| level.registry().get(level.block(cell[0], y, cell[1]))).flatten()?;
        let top = y as f32 + state.boxes.iter().map(|aabb| aabb.max[1]).fold(f32::NAN, f32::max);
        (top <= from && top >= to).then_some(top)
    })
}

fn centre(pos: BlockPos) -> Vec3f {
    let [x, y, z] = pos.map(|c| c as f32 + 0.5);
    Vec3f { x, y, z }
}

/// The falling block `block` breaks at `cell`: its own item, not its loot, and its fragments
/// (BDS `FallingBlockActor::breakBlock`).
fn break_up(world: &mut World, layer: Entity, block: u32, cell: BlockPos) {
    let Some((name, hash)) = level(world, layer).and_then(|level| Some((level.registry().get(block)?.name, level.registry().network_hash(block)))) else { return };
    // A brushable block shatters, leaving nothing.
    if !name.contains("suspicious_") {
        loot::spill(world, layer, cell, &[Item::new(if name == SNOW { "minecraft:snowball" } else { name }, 1)]);
    }
    let particles = sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleDestroy, position: centre(cell), data: hash as i32 });
    send::to_viewers(world, layer, cell, &[particles]);
}

/// Puts the block into the cell it landed in when that gives way, else breaks it up there; the
/// cell, when the block is in it.
fn land(world: &mut World, layer: Entity, block: u32, cell: BlockPos) -> Option<BlockPos> {
    let (registry, here) = self::block(world, layer, cell)?;
    let name = registry.get(block)?.name;
    if name.contains("suspicious_") || !gives_way(registry, here) || !send::set(world, layer, cell, block) {
        break_up(world, layer, block, cell);
        return None;
    }
    if name.ends_with("anvil") {
        let thud = sapling_net::encode(&LevelEvent { event: LevelEventEvent::SoundAnvilFall, position: centre(cell), data: 0 });
        send::to_viewers(world, layer, cell, &[thud]);
    }
    Some(cell)
}

fn moved(id: u64, [_, y, _]: [f32; 3], on_ground: bool, tick: u64) -> Bytes {
    sapling_net::encode(&MoveEntityDelta {
        runtime_entity_id: id,
        x: None,
        y: Some(y),
        z: None,
        rot_x: None,
        rot_y: None,
        rot_z: None,
        on_ground,
        force_move: false,
        force_move_local_entity: false,
        force_completion: false,
        ticks: tick,
    })
}

/// One tick of a falling block (BDS `FallingBlockActor::normalTick`): it speeds up, drops and
/// lands on the first collision box it meets. The cell it was placed in, when it was.
fn fall(world: &mut World, entity: Entity) -> Option<BlockPos> {
    let tick = world.resource::<Tick>().0;
    let id = world.get::<NetId>(entity)?.0;
    if let Some(waited) = world.get::<FallingBlock>(entity)?.landed_for {
        match waited + 1 >= LINGER {
            true => mobs::remove(world, entity),
            false => world.get_mut::<FallingBlock>(entity)?.landed_for = Some(waited + 1),
        }
        return None;
    }
    let mut falling = world.get_mut::<FallingBlock>(entity)?;
    falling.airborne += 1;
    falling.fall_speed += GRAVITY;
    let (layer, block, from, speed, airborne) = (falling.layer, falling.block, falling.at, falling.fall_speed, falling.airborne);
    let floor = floor_between(world, layer, from, from[1] - speed);
    let at = [from[0], floor.unwrap_or(from[1] - speed), from[2]];
    let mut falling = world.get_mut::<FallingBlock>(entity)?;
    falling.at = at;
    falling.fall_speed = if floor.is_some() { 0.0 } else { speed * DRAG };
    falling.landed_for = floor.map(|_| 0);
    let velocity = Vec3f { x: 0.0, y: -falling.fall_speed, z: 0.0 };
    let hash = level(world, layer).map_or(0, |level| level.registry().network_hash(block));
    let shown = world.get::<FallingBlock>(entity).map(|falling| announcement(id, falling, hash))?;
    mobs::announce_as(world, entity, shown);
    let cell = at.map(|c| c.floor() as i32);
    let motion = sapling_net::encode(&SetEntityMotion { runtime_entity_id: id, velocity, tick });
    send::to_viewers(world, layer, cell, &[moved(id, wire(at), floor.is_some(), tick), motion]);
    let below_world = level(world, layer).is_some_and(|level| !level.in_height(cell[1]) && !level.in_height(cell[1] + 1));
    match floor {
        Some(_) => land(world, layer, block, cell),
        None if airborne > MAX_AIRBORNE || airborne > MAX_BELOW_WORLD && below_world => {
            mobs::remove(world, entity);
            if !below_world {
                break_up(world, layer, block, cell);
            }
            None
        }
        None => None,
    }
}

/// Moves the falling blocks and starts the falls that are due; the cells that changed, for their
/// neighbours to settle.
pub(super) fn tick(world: &mut World) -> Vec<(Entity, BlockPos)> {
    let falling: Vec<(Entity, Entity)> = world.query::<(Entity, &FallingBlock)>().iter(world).map(|(entity, falling)| (entity, falling.layer)).collect();
    let mut changed: Vec<(Entity, BlockPos)> = falling.into_iter().filter_map(|(entity, layer)| Some((layer, fall(world, entity)?))).collect();
    let now = world.resource::<Tick>().0;
    let Some(mut slides) = world.get_resource_mut::<Slides>() else { return changed };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut slides.0).into_iter().partition(|(when, ..)| *when <= now);
    slides.0 = waiting;
    changed.extend(due.into_iter().filter_map(|(_, layer, pos)| Some((layer, slide(world, layer, pos)?))));
    changed
}

