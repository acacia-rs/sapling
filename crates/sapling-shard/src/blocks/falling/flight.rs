//! A falling block in the air: BDS `FallingBlockActor::normalTick`, with the packets and timing
//! a BDS was recorded sending (research/captures/bds-mechanisms-findings.md "Falling blocks").

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{BlockPos, Item};

use super::super::place::gives_way;
use super::super::{level, loot, send};
use super::{SNOW, block};
use crate::body::{Body, Physics};
use crate::components::NetId;
use crate::mobs;
use crate::sync::Told;

/// Its box is 0.98 wide and high.
const SIZE: f32 = 0.98;
/// Ticks in the air before it gives up and leaves its item; below the world, before it vanishes.
const MAX_AIRBORNE: u32 = 600;
const MAX_BELOW_WORLD: u32 = 100;
/// Ticks from landing to the block being placed, and from then to the entity going.
const PLACE_AFTER: u32 = 1;
const LINGER: u32 = 3;
/// The wire position is this far above the bottom of its 0.98-high box.
const WIRE_OFFSET: f32 = 0.49;
/// BDS spawns it at the middle of the cell it leaves: its box starts this far above the floor.
const SPAWN_LIFT: f32 = 0.01;

#[derive(Component)]
pub(crate) struct FallingBlock {
    layer: Entity,
    block: u32,
    body: Body,
    told: Told,
    airborne: u32,
    landed_for: Option<u32>,
}

impl FallingBlock {
    /// The block `block` leaving `cell`, a tick into its fall, as BDS first shows one.
    pub(super) fn leaving(level: &sapling_world::World, layer: Entity, block: u32, [x, y, z]: BlockPos) -> Self {
        let mut body = Body::new([x as f32 + 0.5, y as f32 + SPAWN_LIFT, z as f32 + 0.5], [0.0; 3], [SIZE, SIZE]);
        body.step(level, &Physics::FALLING_BLOCK);
        Self { layer, block, body, told: Told::new(wire(body.at)), airborne: 1, landed_for: None }
    }

    pub(super) fn at(&self) -> [f32; 3] {
        self.body.at
    }

    pub(super) fn announcement(&self, id: u64, hash: u32) -> Vec<Bytes> {
        vec![sapling_net::encode(&sapling_data::add_falling_block(id, wire(self.body.at), self.body.motion, hash))]
    }
}

fn wire([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, y + WIRE_OFFSET, z]
}

fn cell_of(at: [f32; 3]) -> BlockPos {
    at.map(|c| c.floor() as i32)
}

fn event(event: LevelEventEvent, [x, y, z]: [f32; 3], data: i32) -> Bytes {
    sapling_net::encode(&LevelEvent { event, position: Vec3f { x, y, z }, data })
}

/// The falling block `block` breaks where it is: its own item, not its loot, and its fragments
/// at the entity (BDS `FallingBlockActor::breakBlock`). A brushable block leaves nothing.
fn break_up(world: &mut World, layer: Entity, block: u32, bottom: [f32; 3]) {
    let Some((name, hash)) = level(world, layer).and_then(|level| Some((level.registry().get(block)?.name, level.registry().network_hash(block)))) else { return };
    let cell = cell_of(bottom);
    send::to_viewers(world, layer, cell, &[event(LevelEventEvent::ParticleDestroy, wire(bottom), hash as i32)]);
    if !name.contains("suspicious_") {
        loot::spill(world, layer, cell, &[Item::new(if name == SNOW { "minecraft:snowball" } else { name }, 1)]);
    }
}

/// Whether the cell a falling block came to rest in takes it.
fn takes(world: &World, layer: Entity, block: u32, cell: BlockPos) -> bool {
    let Some((registry, here)) = super::block(world, layer, cell) else { return false };
    gives_way(registry, here) && registry.get(block).is_some_and(|state| !state.name.contains("suspicious_"))
}

/// One tick of a falling block; the cell its block was placed in, when it was.
fn step(world: &mut World, entity: Entity) -> Option<BlockPos> {
    let id = world.get::<NetId>(entity)?.0;
    let falling = world.get::<FallingBlock>(entity)?;
    let (layer, block_id, mut body) = (falling.layer, falling.block, falling.body);
    if let Some(waited) = falling.landed_for.map(|waited| waited + 1) {
        world.get_mut::<FallingBlock>(entity)?.landed_for = Some(waited);
        if waited >= PLACE_AFTER + LINGER {
            mobs::remove(world, entity);
        }
        let cell = cell_of(body.at);
        return (waited == PLACE_AFTER && send::set(world, layer, cell, block_id)).then_some(cell);
    }
    let airborne = falling.airborne + 1;
    body.step(level(world, layer)?, &Physics::FALLING_BLOCK);
    let (at, landed) = (body.at, body.on_ground);
    let below_world = level(world, layer).is_some_and(|level| !level.in_height(cell_of(at)[1]) && !level.in_height(cell_of(at)[1] + 1));
    let refused = landed && !takes(world, layer, block_id, cell_of(at));
    if refused || !landed && (airborne > MAX_AIRBORNE || airborne > MAX_BELOW_WORLD && below_world) {
        mobs::remove(world, entity);
        if !below_world {
            break_up(world, layer, block_id, at);
        }
        return None;
    }
    if landed {
        body.motion = [0.0; 3];
    }
    let mut falling = world.get_mut::<FallingBlock>(entity)?;
    (falling.body, falling.airborne, falling.landed_for) = (body, airborne, landed.then_some(0));
    let mut packets = falling.told.update(id, wire(at), body.motion, landed);
    let hash = level(world, layer).map_or(0, |level| level.registry().network_hash(block_id));
    let shown = world.get::<FallingBlock>(entity)?.announcement(id, hash);
    mobs::reannounce(world, entity, shown);
    if landed {
        let is_anvil = block(world, layer, cell_of(at)).and_then(|(registry, _)| registry.get(block_id)).is_some_and(|state| state.name.ends_with("anvil"));
        if is_anvil {
            packets.push(event(LevelEventEvent::SoundAnvilFall, cell_of(at).map(|c| c as f32), 0));
        }
    }
    send::to_viewers(world, layer, cell_of(at), &packets);
    None
}

/// Moves every falling block a tick; the cells blocks were placed in.
pub(super) fn fly(world: &mut World) -> Vec<(Entity, BlockPos)> {
    let falling: Vec<(Entity, Entity)> = world.query::<(Entity, &FallingBlock)>().iter(world).map(|(entity, falling)| (entity, falling.layer)).collect();
    falling.into_iter().filter_map(|(entity, layer)| Some((layer, step(world, entity)?))).collect()
}
