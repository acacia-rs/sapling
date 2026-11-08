//! A falling block in the air: BDS `FallingBlockActor::normalTick`, with the packets and timing
//! a BDS was recorded sending (research/captures/bds-mechanisms-findings.md "Falling blocks").

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{BlockPos, Item};

use super::super::place::gives_way;
use super::super::{level, loot, send};
use super::{SNOW, block, dripstone, is_water, set_wet};
use crate::body::{Body, Physics};
use crate::components::{InLayer, NetId};
use crate::{entities, mobs};

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
const ANVIL_HARM: f32 = 2.0;
const ANVIL_MOST: f32 = 40.0;
const PLAYER: [f32; 2] = [0.6, 1.8];

/// A falling block's own part; its [`Body`] and what viewers were told are beside it (entities.rs).
#[derive(Component)]
pub(crate) struct FallingBlock {
    block: u32,
    airborne: u32,
    landed_for: Option<u32>,
    refused: bool,
    /// The height its fall began at: the floor of the cell it left.
    from: f32,
    /// What it costs what it lands on for each block it fell, past the first.
    harm: f32,
}

/// Has the falling block `entity` hurt what it lands on, `per_block` for each block it fell.
pub(super) fn hurts(world: &mut World, entity: Entity, per_block: f32) {
    if let Some(mut falling) = world.get_mut::<FallingBlock>(entity) {
        falling.harm = per_block;
    }
}

const KIND: &str = "minecraft:falling_block";

fn announcement(id: u64, body: &Body, hash: u32) -> Vec<Bytes> {
    vec![sapling_net::encode(&sapling_data::add_falling_block(id, wire(body.at), body.motion, hash))]
}

/// Sets the block `block` falling out of `cell`, shown a tick into its fall as BDS first shows one,
/// for the world folder to keep with the column it is over (actors.rs).
pub(super) fn start(world: &mut World, layer: Entity, block: u32, [x, y, z]: BlockPos) -> Option<Entity> {
    let (body, hash) = {
        let level = level(world, layer)?;
        let mut body = Body::new([x as f32 + 0.5, y as f32 + SPAWN_LIFT, z as f32 + 0.5], [0.0; 3], [SIZE, SIZE]);
        // A falling block draws on no chance.
        body.step(level, &Physics::FALLING_BLOCK, &mut || 0.5);
        (body, level.registry().network_hash(block))
    };
    let entity = spawn(world, layer, block, hash, body, 1)?;
    let unique_id = entities::new_unique_id(world, entity)?;
    if let Some(compound) = sapling_world::new_falling_block_actor(unique_id, hash, level(world, layer)?.registry(), body.at, body.motion, 1) {
        entities::keep(world, entity, layer, unique_id, compound);
    }
    Some(entity)
}

fn spawn(world: &mut World, layer: Entity, block: u32, hash: u32, body: Body, airborne: u32) -> Option<Entity> {
    let entity = entities::spawn(world, layer, KIND, body, wire(body.at), |id| Some(announcement(id, &body, hash)))?;
    entities::flag_moving(world, entity, true);
    // A new one is a tick into its fall, under the floor of its cell.
    let from = body.at[1].round();
    world.entity_mut(entity).insert(FallingBlock { block, airborne, landed_for: None, refused: false, from, harm: 0.0 });
    Some(entity)
}

/// A landing anvil that fell `fallen` hurts the players and mobs its box meets: two for each block
/// past the first, at most 40 (research/captures/bds-anvil.txt: 6 blocks cost 10, 16 killed; a
/// helmet's share is vitals' `HELMETED`, bds-i1anvil.txt).
fn crush(world: &mut World, layer: Entity, body: &Body, fallen: f32) {
    let amount = ((fallen - 1.0).ceil() * ANVIL_HARM).clamp(0.0, ANVIL_MOST);
    dripstone::wound_mobs(world, layer, body, amount, crate::creatures::Cause::Anvil);
    let (low, high) = body.bounds();
    let meets = |feet: [f32; 3]| (feet[0] - body.at[0]).abs() < (SIZE + PLAYER[0]) / 2.0 && (feet[2] - body.at[2]).abs() < (SIZE + PLAYER[0]) / 2.0 && feet[1] < high[1] && low[1] < feet[1] + PLAYER[1];
    let struck: Vec<Entity> = crate::components::in_world(world)
        .into_iter()
        .filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer) && world.get::<crate::components::Position>(*player).is_some_and(|position| meets(position.feet)))
        .collect();
    for player in struck.into_iter().filter(|_| amount > 0.0) {
        crate::vitals::crushed(world, player, amount);
    }
}

/// A landing anvil that fell `fallen` comes down one step more worn, 1 in 20 for each block past the
/// first: fitted to bds-i1chip.txt (10 of 92 chipped falling 3, 43 of 92 falling 11; Java's extra
/// 1 in 20 fit worse). A damaged one lands as it is (Java breaks it; not drilled).
fn chip(world: &mut World, entity: Entity, layer: Entity, anvil: u32, fallen: f32) {
    const WORN: [(&str, &str); 2] = [("minecraft:anvil", "minecraft:chipped_anvil"), ("minecraft:chipped_anvil", "minecraft:damaged_anvil")];
    let chance = (fallen - 1.0).ceil().max(0.0) * 0.05;
    let Some(registry) = level(world, layer).map(|level| level.registry()) else { return };
    let Some(state) = registry.get(anvil) else { return };
    let Some(worn) = WORN.iter().find(|(name, _)| *name == state.name).and_then(|(_, worn)| registry.find(worn, state.properties)) else { return };
    if world.get_resource_or_insert_with(super::super::Dice::default).fraction() < chance
        && let Some(mut falling) = world.get_mut::<FallingBlock>(entity)
    {
        falling.block = worn;
    }
}

/// Writes each kept falling block's time in the air into what the world folder keeps of it.
pub(crate) fn keep_times(world: &mut World) {
    let falling: Vec<(Entity, u32)> = world.query_filtered::<(Entity, &FallingBlock), With<entities::Stored>>().iter(world).map(|(entity, falling)| (entity, falling.airborne)).collect();
    for (entity, time) in falling {
        entities::restore(world, entity, |compound, _| sapling_world::timed_falling_block(compound, time).ok());
    }
}

/// Makes a falling block a world folder kept (its actor compound `compound`) fall in `layer`
/// again; false for one whose block the registry does not know.
pub(crate) fn from_storage(world: &mut World, layer: Entity, at: [f32; 3], unique_id: i64, compound: Vec<u8>) -> bool {
    let Some((hash, time)) = sapling_world::falling_block(&compound) else { return false };
    let Some(block) = level(world, layer).and_then(|level| level.registry().runtime_id_from_hash(hash)) else { return false };
    let Some(entity) = spawn(world, layer, block, hash, Body::new(at, [0.0; 3], [SIZE, SIZE]), u32::from(time)) else { return false };
    entities::keep(world, entity, layer, unique_id, compound);
    true
}

fn is_dripstone(world: &World, layer: Entity, block: u32) -> bool {
    level(world, layer).is_some_and(|level| dripstone::is_dripstone(level.registry(), block))
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

/// The fragments of the falling block `block` breaking at the entity (BDS
/// `FallingBlockActor::breakBlock`).
fn shatter(world: &mut World, layer: Entity, block: u32, bottom: [f32; 3]) {
    let Some(hash) = level(world, layer).map(|level| level.registry().network_hash(block)) else { return };
    send::to_viewers(world, layer, cell_of(bottom), &[event(LevelEventEvent::ParticleDestroy, wire(bottom), hash as i32)]);
}

/// What a broken falling block leaves: its own item, not its loot; a brushable block nothing.
fn leave_item(world: &mut World, layer: Entity, block: u32, bottom: [f32; 3]) {
    let Some(name) = level(world, layer).and_then(|level| level.registry().get(block)).map(|state| state.name) else { return };
    if !name.contains("suspicious_") {
        loot::let_go(world, layer, wire(bottom), &[Item::new(if name == SNOW { "minecraft:snowball" } else { name }, 1)]);
    }
}

/// Whether the cell a falling block came to rest in takes it.
fn takes(world: &World, layer: Entity, cell: BlockPos) -> bool {
    super::block(world, layer, cell).is_some_and(|(registry, here)| gives_way(registry, here))
}

fn is_brushable(world: &World, layer: Entity, block: u32) -> bool {
    level(world, layer).and_then(|level| level.registry().get(block)).is_some_and(|state| state.name.contains("suspicious_"))
}

/// A brushable block breaks as it lands (bds-s3misc.txt): dust at its cell's corner, named by the
/// block's default state whatever its own, then put and broken the next tick, leaving nothing.
fn brushable_dust(world: &World, layer: Entity, block: u32, cell: BlockPos) -> Option<Bytes> {
    let registry = level(world, layer)?.registry();
    let default = send::resolve(registry, &sapling_api::Block::new(registry.get(block)?.name))?;
    Some(event(LevelEventEvent::ParticleDestroy, cell.map(|c| c as f32), registry.network_hash(default) as i32))
}

/// One tick of a falling block; the cell its block was placed in, when it was.
fn step(world: &mut World, entity: Entity, layer: Entity, id: u64) -> Option<BlockPos> {
    let falling = world.get::<FallingBlock>(entity)?;
    let (block_id, airborne) = (falling.block, falling.airborne + 1);
    if falling.refused {
        let at = world.get::<Body>(entity)?.at;
        mobs::remove(world, entity);
        leave_item(world, layer, block_id, at);
        return None;
    }
    if let Some(waited) = falling.landed_for.map(|waited| waited + 1) {
        world.get_mut::<FallingBlock>(entity)?.landed_for = Some(waited);
        let cell = cell_of(world.get::<Body>(entity)?.at);
        if waited >= PLACE_AFTER + LINGER {
            mobs::remove(world, entity);
        }
        if waited != PLACE_AFTER {
            return None;
        }
        if is_dripstone(world, layer, block_id) {
            world.entity_mut(entity).remove::<entities::Stored>();
            dripstone::shatter(world, layer, block_id, cell);
            return None;
        }
        if is_brushable(world, layer, block_id) {
            world.entity_mut(entity).remove::<entities::Stored>();
            let Some(here) = level(world, layer).filter(|level| level.in_height(cell[1])).map(|level| level.block(cell[0], cell[1], cell[2])) else { return None };
            send::show_states(world, layer, &[(cell, block_id), (cell, here)]);
            return None;
        }
        if !send::set_quietly(world, layer, cell, block_id) {
            return None;
        }
        world.entity_mut(entity).remove::<entities::Stored>();
        send::tell_synced(world, layer, cell, id, send::Transition::Destroy);
        return Some(cell);
    }
    let (body, mut packets) = entities::step(world, entity, layer, id, &Physics::FALLING_BLOCK, WIRE_OFFSET)?;
    let (at, landed) = (body.at, body.on_ground);
    // Concrete powder whose middle is in water that touches more water sets there and is gone at
    // once (BDS `ConcretePowderBlock::shouldStopFalling`; research/captures/bds-concrete.txt, the
    // two-deep pool); in a pool one deep it lands as powder.
    let middle = cell_of(wire(at));
    if is_water(world, layer, middle) && set_wet(world, layer, middle, block_id) {
        send::to_viewers(world, layer, middle, &packets);
        mobs::remove(world, entity);
        return Some(middle);
    }
    let below_world = level(world, layer).is_some_and(|level| !level.in_height(cell_of(at)[1]) && !level.in_height(cell_of(at)[1] + 1));
    // Refused: it shatters now and goes, leaving its item, the next tick, as BDS was seen to.
    let is_dripstone = is_dripstone(world, layer, block_id);
    if landed && !is_dripstone && !takes(world, layer, cell_of(at)) {
        shatter(world, layer, block_id, at);
        world.get_mut::<FallingBlock>(entity)?.refused = true;
        return None;
    }
    if !landed && (airborne > MAX_AIRBORNE || airborne > MAX_BELOW_WORLD && below_world) {
        mobs::remove(world, entity);
        if !below_world {
            shatter(world, layer, block_id, at);
            leave_item(world, layer, block_id, at);
        }
        return None;
    }
    let mut falling = world.get_mut::<FallingBlock>(entity)?;
    (falling.airborne, falling.landed_for) = (airborne, landed.then_some(0));
    let (fell, harm) = (falling.from - at[1], falling.harm);
    if !packets.is_empty() {
        let hash = level(world, layer).map_or(0, |level| level.registry().network_hash(block_id));
        mobs::reannounce(world, entity, announcement(id, &body, hash));
    }
    // BDS tells the health of whom it landed on after the landing's events (bds-v2drip.txt, bds-z2open.txt).
    let (_, hurt) = crate::vitals::health_held(world, |world| {
        if !landed {
            return;
        }
        let is_anvil = block(world, layer, cell_of(at)).and_then(|(registry, _)| registry.get(block_id)).is_some_and(|state| state.name.ends_with("anvil"));
        // BDS sends the sound before the landing move, after the hurt it did.
        if is_anvil {
            crush(world, layer, &body, fell);
            chip(world, entity, layer, block_id, fell);
            packets.insert(0, event(LevelEventEvent::SoundAnvilFall, cell_of(at).map(|c| c as f32), 0));
        }
        if is_dripstone {
            dripstone::hurt(world, layer, &body, harm, fell);
            packets.splice(0..0, dripstone::landed(world, layer, block_id, cell_of(at)));
        }
        if is_brushable(world, layer, block_id)
            && let Some(dust) = brushable_dust(world, layer, block_id, cell_of(at))
        {
            packets.insert(0, dust);
        }
    });
    send::to_viewers(world, layer, cell_of(at), &packets);
    crate::vitals::tell_held_health(world, hurt);
    None
}

/// Moves every falling block whose cell ticks a tick; the cells blocks were placed in. BDS's sand out of
/// the tick region hung where it was and fell on when the players came back (bds-tickqueue.txt at 116183).
pub(super) fn fly(world: &mut World) -> Vec<(Entity, BlockPos)> {
    let region = crate::ticking::Region::of(world);
    let falling: Vec<(Entity, Entity, u64)> = world
        .query_filtered::<(Entity, &InLayer, &NetId, &Body), With<FallingBlock>>()
        .iter(world)
        .filter(|(_, layer, _, body)| region.ticks_block(world, layer.0, cell_of(body.at)))
        .map(|(entity, layer, id, _)| (entity, layer.0, id.0))
        .collect();
    falling.into_iter().filter_map(|(entity, layer, id)| Some((layer, step(world, entity, layer, id)?))).collect()
}
