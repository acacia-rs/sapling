//! Beehives and bee nests holding bees, as BDS's held them (research/captures/bds-keephive.txt,
//! bds-bees.txt): a bee that reaches one goes in while it holds fewer than three, heard
//! `block.beehive.enter` at the block's corner, and stays 600 ticks, or 2400 with pollen. Its time
//! up, it comes out in front once it is day and dry, adding a level of honey if it brought pollen.
//! While bees are in, `block.beehive.work` is heard from the middle now and then. The hive's entity
//! (`sapling_world::occupied_hive`) is written only as bees come and go, so its `TicksLeftToStay`
//! are those of the last write, as BDS's were.
// TODO: a hive broken with silk touch lets its bees out calm instead of keeping them in its item;
// a hive on fire or a campfire under one is not looked at; a nest from generation holds no bees.

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ChunkPos, Occupant};

use super::super::Health;
use crate::blocks::{self, Dice};
use crate::components::{InLayer, Layer, Tick};
use crate::entities::Stored;
use crate::view::View;

const ENTITY: &str = "Beehive";
const HOLDS: usize = 3;
/// Ticks a bee stays in without pollen and with it.
const STAYS: [i32; 2] = [600, 2400];
const HONEY: &str = "honey_level";
const HONEY_FULL: u8 = 5;
/// The chance a tick of `block.beehive.work` from a hive with bees in.
const WORKS: f32 = 0.005;
/// Columns this many chunks across from a player's tick their hives (BDS's `tick-distance`).
const REACH: i32 = 4;

/// The tick each hive's entity was last written in, by world and place: its bees' `TicksLeftToStay`
/// count from then.
#[derive(Resource, Default)]
struct Written(HashMap<(Entity, BlockPos), u64>);

pub(crate) fn is_hive(name: &str) -> bool {
    matches!(name, "minecraft:beehive" | "minecraft:bee_nest")
}

fn level(world: &World, layer: Entity) -> Option<&sapling_world::World> {
    world.get::<Layer>(layer).map(|layer| &layer.world)
}

/// The hive's entity at `pos` and its bees with the ticks they have left now.
fn held(world: &World, layer: Entity, pos: BlockPos) -> Option<(Value, Vec<Occupant>)> {
    let level = level(world, layer)?;
    let state = level.registry().get(level.block(pos[0], pos[1], pos[2]))?;
    if !is_hive(state.name) {
        return None;
    }
    let entity = level.block_entity(pos).map_or_else(|| sapling_world::new_hive(pos), |entity| entity.nbt.clone());
    let now = world.resource::<Tick>().0;
    let since = world.get_resource::<Written>().and_then(|written| written.0.get(&(layer, pos)).copied()).map_or(0, |at| now.saturating_sub(at) as i32);
    let mut occupants = sapling_world::hive_occupants(&entity);
    for occupant in &mut occupants {
        occupant.ticks_left -= since;
    }
    Some((entity, occupants))
}

fn write(world: &mut World, layer: Entity, pos: BlockPos, entity: &Value, occupants: &[Occupant]) {
    let now = world.resource::<Tick>().0;
    let kept: Vec<Occupant> = occupants.iter().cloned().map(|occupant| Occupant { ticks_left: occupant.ticks_left.max(0), ..occupant }).collect();
    if let Some(mut level) = world.get_mut::<Layer>(layer) {
        level.world.set_block_entity(pos, &sapling_world::occupied_hive(entity, &kept));
    }
    world.get_resource_or_insert_with(Written::default).0.insert((layer, pos), now);
}

fn heard(world: &mut World, layer: Entity, pos: BlockPos, name: &str, at: [f32; 3]) {
    let packet = blocks::noise(name, Vec3f { x: at[0], y: at[1], z: at[2] }, -1);
    blocks::to_viewers(world, layer, pos, &[packet]);
}

/// The bee goes into the hive at `pos`, carrying pollen or not; false where it is no hive or is full.
pub(super) fn enter(world: &mut World, bee: Entity, layer: Entity, pos: BlockPos, nectar: bool) -> bool {
    let Some((entity, mut occupants)) = held(world, layer, pos) else { return false };
    if occupants.len() >= HOLDS {
        return false;
    }
    let mut kept = world.query::<(&crate::body::Body, &Stored, Option<&Health>)>();
    let Some(actor) = kept.get(world, bee).ok().and_then(|(body, stored, health)| crate::actors::compound_now(body, stored, health, None)) else { return false };
    occupants.push(Occupant { actor, ticks_left: STAYS[usize::from(nectar)], nectar });
    write(world, layer, pos, &entity, &occupants);
    crate::mobs::remove(world, bee);
    heard(world, layer, pos, "block.beehive.enter", pos.map(|c| c as f32));
    true
}

/// Where a bee comes out of the hive at `pos` whose block is `id`, and its yaw: the middle of the
/// cell in front of its face, turned away from it (bds-hives.txt at 56583: `direction` 1, yaw 90).
fn front(registry: &sapling_world::BlockRegistry, id: u32, pos: BlockPos) -> ([f32; 3], f32) {
    let facing = registry.get(id).and_then(|state| state.properties.split(',').find_map(|pair| pair.strip_prefix("direction="))).and_then(|value| value.parse::<u8>().ok()).unwrap_or(0);
    let ([dx, dz], yaw) = match facing {
        0 => ([0, 1], 0.0),
        1 => ([-1, 0], 90.0),
        2 => ([0, -1], 180.0),
        _ => ([1, 0], -90.0),
    };
    ([pos[0] + dx, pos[1], pos[2] + dz].map(|c| c as f32 + 0.5), yaw)
}

/// Lets `occupant` out of the hive at `pos` at `at` turned to `yaw`; the bee, made anew from its compound.
fn release(world: &mut World, layer: Entity, pos: BlockPos, occupant: &Occupant, at: [f32; 3], yaw: f32) -> Option<Entity> {
    let (_, Some(unique_id), _) = sapling_world::stored_actors(&occupant.actor).ok()?.into_iter().next()? else { return None };
    let compound = sapling_world::placed_actor(&occupant.actor, unique_id, at, [yaw, 0.0]).ok()?;
    let (Some(actor), ..) = sapling_world::stored_actors(&compound).ok()?.into_iter().next()? else { return None };
    if !super::super::from_storage(world, layer, actor, unique_id, compound) {
        return None;
    }
    let bee = world.query::<(Entity, &crate::tags::UniqueId)>().iter(world).find(|(_, kept)| kept.0 == unique_id).map(|(entity, _)| entity)?;
    super::came_home(world, bee, pos);
    if occupant.nectar {
        add_honey(world, layer, pos);
    }
    Some(bee)
}

fn add_honey(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let id = level.block(pos[0], pos[1], pos[2]);
    let honey = level.registry().get(id).and_then(|state| state.properties.split(',').find_map(|pair| pair.strip_prefix("honey_level="))).and_then(|value| value.parse::<u8>().ok());
    let Some(more) = honey.filter(|honey| *honey < HONEY_FULL).and_then(|honey| blocks::with_property(level.registry(), id, HONEY, &(honey + 1).to_string())) else { return };
    blocks::set(world, layer, pos, more);
}

/// The bees in the hive at `pos`, taken out of it before it breaks.
pub(crate) fn taken(world: &World, layer: Entity, pos: BlockPos) -> Vec<Occupant> {
    held(world, layer, pos).map(|(_, occupants)| occupants).unwrap_or_default()
}

/// The block `name` at `pos`, holding `bees`, was broken with `silk_touch` or not: if it was a hive
/// its bees come out, and they and every bee within 16 of it go for the nearest player within 10
/// of them (the pack's `block_sensor` and `minecraft:exited_disturbed_hive`).
pub(crate) fn broken(world: &mut World, layer: Entity, pos: BlockPos, name: &str, bees: Vec<Occupant>, silk_touch: bool) {
    if !is_hive(name) {
        return;
    }
    world.get_resource_or_insert_with(Written::default).0.remove(&(layer, pos));
    // Out of the gone block's middle, as BDS's came (bds-bees.txt at 110177).
    for occupant in &bees {
        release(world, layer, pos, occupant, pos.map(|c| c as f32 + 0.5), 0.0);
    }
    if !silk_touch {
        super::disturbed(world, layer, pos);
    }
}

/// The hives of the columns about players: their bees count down and come out when due, and
/// they are heard working.
pub(crate) fn tick(world: &mut World) {
    let mut columns = Vec::new();
    for (in_layer, view) in world.query::<(&InLayer, &View)>().iter(world) {
        let Some(center) = view.center() else { continue };
        columns.extend((-REACH..=REACH).flat_map(|dx| (-REACH..=REACH).map(move |dz| (in_layer.0, ChunkPos::new(center.x + dx, center.z + dz)))));
    }
    columns.sort_by_key(|(layer, pos)| (*layer, pos.x, pos.z));
    columns.dedup();
    let mut hives = Vec::new();
    for (layer, column) in columns {
        let Some(level) = level(world, layer) else { continue };
        let occupied = |entity: &&sapling_world::BlockEntity| matches!(entity.nbt.get("id"), Some(Value::String(id)) if id.as_str() == ENTITY) && matches!(entity.nbt.get("Occupants"), Some(Value::List(list)) if !list.items.is_empty());
        hives.extend(level.block_entities_in(column).into_iter().filter(occupied).map(|entity| (layer, entity.pos)));
    }
    for (layer, pos) in hives {
        let Some((entity, occupants)) = held(world, layer, pos) else { continue };
        if world.get_resource_or_insert_with(Dice::default).fraction() < WORKS {
            heard(world, layer, pos, "block.beehive.work", pos.map(|c| c as f32 + 0.5));
        }
        if !super::may_fly(world, layer) || occupants.iter().all(|occupant| occupant.ticks_left > 0) {
            continue;
        }
        let (due, staying): (Vec<Occupant>, Vec<Occupant>) = occupants.into_iter().partition(|occupant| occupant.ticks_left <= 0);
        write(world, layer, pos, &entity, &staying);
        let Some(level) = level(world, layer) else { continue };
        let (at, yaw) = front(level.registry(), level.block(pos[0], pos[1], pos[2]), pos);
        for occupant in &due {
            if release(world, layer, pos, occupant, at, yaw).is_some() {
                heard(world, layer, pos, "block.beehive.exit", pos.map(|c| c as f32));
            }
        }
    }
}
