//! End crystals, as BDS made and broke them (research/captures/bds-dragon-again.txt): one put on
//! obsidian or bedrock stands a block up in the middle of the cell, announced with its metadata
//! twice; in the End it lights fire in its cell; a spike's shows its base and is turned at random.
//! Anything that hurts one blows it up at power 6, breaking nothing (ender_crystal.json,
//! research/dragon-fight-decompile.md 7). Its beam points at a block.

use acacia_proto::packets::{AddEntity, LevelEventGeneric, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3i};
use acacia_proto::nbt::{Nbt, Value as NbtValue};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::BlockPos;
use sapling_data::AddEntityOptions;

use crate::body::Body;
use crate::components::{InLayer, Layer, NetId};
use crate::entities;

pub(crate) const KIND: &str = "minecraft:ender_crystal";
const POWER: f32 = 6.0;
/// `LevelEventGeneric` 2026, an explosion's particles, zigzagged as acacia-proto does not (sleep.rs).
const EXPLOSION_PARTICLES: u32 = 2026 << 1;
/// Blocks one may stand on.
const BASES: [&str; 2] = ["minecraft:obsidian", "minecraft:bedrock"];
const NO_BEAM: BlockPos = [0, 0, 0];

#[derive(Component)]
pub(crate) struct Crystal {
    pub beam: Option<BlockPos>,
}

fn announcement(id: u64, feet: [f32; 3], yaw: f32, base: bool) -> Vec<Bytes> {
    let options = AddEntityOptions { id, identifier: KIND, feet, yaw, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(if base { MetadataFlags1::SHOWBASE.0 } else { 0 }), health: None, name: "" };
    let added: AddEntity = sapling_data::add_entity(&options);
    // BDS sends the whole metadata again at once.
    let again = SetEntityData { runtime_entity_id: id, metadata: added.metadata.clone(), properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    vec![sapling_net::encode(&AddEntity { pitch: 0.0, head_yaw: 0.0, body_yaw: 0.0, ..added }), sapling_net::encode(&again)]
}

/// A crystal standing with its feet at `feet`, turned `yaw`, its base shown or not.
pub(super) fn spawn(world: &mut World, layer: Entity, feet: [f32; 3], yaw: f32, base: bool) -> Option<Entity> {
    let entity = shown(world, layer, feet, yaw, base)?;
    let unique_id = entities::new_unique_id(world, entity)?;
    entities::keep(world, entity, layer, unique_id, sapling_world::new_actor(unique_id, KIND, feet, [0.0; 3]));
    Some(entity)
}

fn shown(world: &mut World, layer: Entity, feet: [f32; 3], yaw: f32, base: bool) -> Option<Entity> {
    let size = sapling_data::mob_size(KIND)?;
    let body = Body::new(feet, [0.0; 3], size);
    let entity = entities::spawn(world, layer, KIND, body, feet, |id| Some(announcement(id, feet, yaw, base)))?;
    world.entity_mut(entity).insert(Crystal { beam: None });
    Some(entity)
}

/// A crystal a world folder kept (its compound `compound`) stands again; a spike's shows its base.
pub(crate) fn from_storage(world: &mut World, layer: Entity, feet: [f32; 3], yaw: f32, unique_id: i64, compound: Vec<u8>) -> bool {
    let base = on_spike(&super::spikes(world, layer), feet);
    let Some(entity) = shown(world, layer, feet, yaw, base) else { return false };
    entities::keep(world, entity, layer, unique_id, compound);
    true
}

/// An end crystal put on the block at `pos`: on obsidian or bedrock with room above, the crystal;
/// false where it may not stand.
pub(crate) fn place(world: &mut World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let named = |y: i32| level.registry().get(level.block(x, y, z)).map(|state| state.name);
    let free = [y + 1, y + 2].iter().all(|&y| level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_air()));
    if !named(y).is_some_and(|name| BASES.contains(&name)) || !free || occupied(world, layer, [x, y + 1, z]) {
        return false;
    }
    let placed = spawn(world, layer, [x as f32 + 0.5, (y + 1) as f32, z as f32 + 0.5], 0.0, false).is_some();
    if placed {
        super::crystal_placed(world, layer);
    }
    placed
}

/// Whether an entity's box reaches into the two cells over `[x, y, z]`'s floor.
fn occupied(world: &mut World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    let (low, high) = ([x as f32, y as f32, z as f32], [x as f32 + 1.0, y as f32 + 2.0, z as f32 + 1.0]);
    world.query::<(&InLayer, &Body)>().iter(world).filter(|(in_layer, _)| in_layer.0 == layer).any(|(_, body)| {
        let (min, max) = body.bounds();
        (0..3).all(|axis| min[axis] < high[axis] && max[axis] > low[axis])
    })
}

/// Points the crystal's beam at `at`, or at nothing.
pub(super) fn set_beam(world: &mut World, crystal: Entity, at: Option<BlockPos>) {
    let (Some(id), Some(layer), Some(body)) = (world.get::<NetId>(crystal).map(|id| id.0), world.get::<InLayer>(crystal).map(|layer| layer.0), world.get::<Body>(crystal).copied()) else { return };
    if let Some(mut kept) = world.get_mut::<Crystal>(crystal) {
        kept.beam = at;
    }
    let [x, y, z] = at.unwrap_or(NO_BEAM);
    let item = MetadataDictionaryItem { key: Key::BlockTarget, r#type: Type::Vec3i, legacy_type: 6, value: Value::Default(Plain::Vec3i(Vec3i { x, y, z })) };
    let told = SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    entities::tell_viewers(world, crystal, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&told)]);
}

/// The particles BDS sends after an explosion's flash (`LevelEventGeneric`).
pub(crate) fn blast_particles([x, y, z]: [f32; 3], power: f32) -> Bytes {
    let float = |name: &str, value: f32| Nbt { name: name.to_owned(), value: NbtValue::Float(value) };
    let nbt = vec![float("originX", x), float("originY", y), float("originZ", z), float("radius", power), Nbt { name: "size".into(), value: NbtValue::Int(0) }];
    sapling_net::encode(&LevelEventGeneric { event_id: EXPLOSION_PARTICLES, nbt })
}

/// The crystal blows up where it stands and is gone: it hurts what is near, breaking nothing.
/// Its flash and particles go at once, the sound the tick after, as BDS's did.
pub(crate) fn burst(world: &mut World, crystal: Entity) {
    let (Some(layer), Some(at)) = (world.get::<InLayer>(crystal).map(|layer| layer.0), world.get::<Body>(crystal).map(|body| body.at)) else { return };
    world.entity_mut(crystal).remove::<Crystal>();
    crate::creatures::explode(world, Some(crystal), layer, at, POWER, crate::creatures::Wreck::Nothing);
    crate::blocks::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[blast_particles(at, POWER)]);
    super::count::crystal_gone(world, layer, crystal, at);
}

/// Whether a crystal standing at `at` is on one of `spikes`.
pub(super) fn on_spike(spikes: &[sapling_worldgen::end::Spike], at: [f32; 3]) -> bool {
    spikes.iter().any(|spike| (at[0] - spike.x as f32 - 0.5).abs() < 1.0 && (at[2] - spike.z as f32 - 0.5).abs() < 1.0)
}

/// Lights fire in each crystal's cell of the End where it is air over a block, as BDS's crystals
/// do every tick (`EnderCrystal::normalTick` 145992a90).
pub(super) fn tick(world: &mut World) {
    let mut crystals = world.query_filtered::<(&InLayer, &Body), With<Crystal>>();
    let cells: Vec<(Entity, BlockPos)> = crystals.iter(world).map(|(layer, body)| (layer.0, body.at.map(|c| c.floor() as i32))).collect();
    for (layer, [x, y, z]) in cells {
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world).filter(|level| level.dimension() == sapling_world::END) else { continue };
        let is_air = |y: i32| level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_air());
        if let Some(fire) = level.registry().find("minecraft:fire", "age=0").filter(|_| is_air(y) && !is_air(y - 1)) {
            crate::blocks::set(world, layer, [x, y, z], fire);
        }
    }
}
