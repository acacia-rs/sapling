//! Blocks on the wire, as BDS sends them (`sapling` example `bds_block_probe`), and between the
//! world's runtime ids and the API's names.

use acacia_proto::nbt::{Nbt, Value};
use acacia_proto::packets::{BlockEntityData, LevelEvent, LevelEventEvent, LevelSoundEvent, UpdateBlock};
use acacia_proto::types::{BlockCoordinates, UpdateBlockFlags, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{Block, BlockPos, Face};
use sapling_world::{BlockEntity, BlockRegistry, ChunkPos};

use crate::components::{InLayer, Layer, Outgoing};

/// BDS sets these two on every UpdateBlock it sends.
const FLAGS: UpdateBlockFlags = UpdateBlockFlags(UpdateBlockFlags::NEIGHBORS.0 | UpdateBlockFlags::NETWORK.0);
/// `entity_unique_id` of a sound no entity made.
const NO_ENTITY: i64 = -1;
const PUNCH: [LevelEventEvent; 6] = [
    LevelEventEvent::ParticlePunchBlockDown,
    LevelEventEvent::ParticlePunchBlockUp,
    LevelEventEvent::ParticlePunchBlockNorth,
    LevelEventEvent::ParticlePunchBlockSouth,
    LevelEventEvent::ParticlePunchBlockWest,
    LevelEventEvent::ParticlePunchBlockEast,
];

pub(crate) fn column([x, _, z]: BlockPos) -> ChunkPos {
    ChunkPos::of_block(x, z)
}

fn centre(pos: BlockPos) -> Vec3f {
    let [x, y, z] = pos.map(|c| c as f32 + 0.5);
    Vec3f { x, y, z }
}

/// A block state as plugins name it.
pub(crate) fn describe(registry: &BlockRegistry, id: u32) -> Block {
    registry.get(id).map_or_else(Block::air, |state| Block { name: state.name.to_owned(), properties: state.properties.to_owned() })
}

/// The runtime id of a named state: the block's first state with the properties it names set, so
/// that `water` with `liquid_depth=3` needs no other.
pub(crate) fn resolve(registry: &BlockRegistry, block: &Block) -> Option<u32> {
    let first = registry.states_of(&block.name).next().map(|(id, _)| id)?;
    if block.properties.is_empty() {
        return Some(first);
    }
    registry.find(&block.name, &block.properties).or_else(|| {
        let mut pairs = block.properties.split(',').map(|pair| pair.split_once('='));
        pairs.try_fold(first, |id, pair| pair.and_then(|(key, value)| super::with_property(registry, id, key, value)))
    })
}

fn update(registry: &BlockRegistry, [x, y, z]: BlockPos, id: u32, layer: u32) -> Bytes {
    sapling_net::encode(&UpdateBlock { position: BlockCoordinates { x, y, z }, block_runtime_id: registry.network_hash(id), flags: FLAGS, layer })
}

/// What a block holds, as clients are shown it.
pub(crate) fn block_entity_data([x, y, z]: BlockPos, entity: &BlockEntity) -> Bytes {
    sapling_net::encode(&BlockEntityData { position: BlockCoordinates { x, y, z }, nbt: Nbt { name: String::new(), value: entity.shown() } })
}

/// The burst of fragments when a block of `id` breaks.
pub(crate) fn destroy_particles(registry: &BlockRegistry, pos: BlockPos, id: u32) -> Bytes {
    sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleDestroy, position: centre(pos), data: registry.network_hash(id) as i32 })
}

/// The few fragments a hit knocks off `face`.
pub(crate) fn punch_particles(registry: &BlockRegistry, pos: BlockPos, face: Face, id: u32) -> Bytes {
    sapling_net::encode(&LevelEvent { event: PUNCH[face as usize], position: centre(pos), data: registry.network_hash(id) as i32 })
}

/// The sound of the block `id` placed: BDS names the block's first state, not the one placed.
pub(crate) fn place_sound(registry: &BlockRegistry, pos: BlockPos, id: u32) -> Bytes {
    let first = registry.get(id).and_then(|state| registry.states_of(state.name).next()).map_or(id, |(first, _)| first);
    sound("place", registry, pos, first)
}

/// The sound `name` of the block `id`, which tells the client what the block is made of.
pub(crate) fn sound(name: &str, registry: &BlockRegistry, pos: BlockPos, id: u32) -> Bytes {
    sound_at(name, registry, centre(pos), id)
}

/// [`sound`] from a point other than the block's centre.
pub(crate) fn sound_at(name: &str, registry: &BlockRegistry, position: Vec3f, id: u32) -> Bytes {
    noise(name, position, registry.network_hash(id) as i32)
}

/// The sound `name` at `position`, with what it says of itself in `extra_data` (-1: nothing).
pub(crate) fn noise(name: &str, position: Vec3f, extra_data: i32) -> Bytes {
    sapling_net::encode(&LevelSoundEvent {
        sound_id: name.into(),
        position,
        extra_data,
        entity_type: String::new(),
        is_baby_mob: false,
        is_global: false,
        entity_unique_id: NO_ENTITY,
        fire_at_position: None,
    })
}

pub(crate) use crate::view::to_viewers;

/// Makes `entity` what the block at `pos` holds and shows it to those who see the block.
pub(crate) fn keep_entity(world: &mut World, layer: Entity, pos: BlockPos, entity: &Value) {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return };
    if !level.world.set_block_entity(pos, entity) {
        return tracing::warn!(?pos, "a block's entity could not be kept");
    }
    let shown: Vec<_> = level.world.block_entity(pos).map(|entity| block_entity_data(pos, entity)).into_iter().collect();
    to_viewers(world, layer, pos, &shown);
}

/// Writes a layer-0 block, and layer 1 as the block decides (waterlog.rs), and shows the layer-0
/// block to everyone who sees the place: their clients move the water between layers by the same
/// rule, and BDS sends nothing for it. False, with nothing changed, where the cell already is so,
/// outside the world's height and where the column is not loaded or cannot be written.
pub(crate) fn set(world: &mut World, layer: Entity, pos: BlockPos, id: u32) -> bool {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return false };
    let [x, y, z] = pos;
    if !level.world.in_height(y) || !level.world.is_loaded(column(pos)) {
        return false;
    }
    let (old, held) = (level.world.block(x, y, z), level.world.liquid(x, y, z));
    let (id, now_held) = super::waterlog::written(level.world.registry(), id, old, held);
    if (id, now_held) == (old, held) || !level.world.set_block(x, y, z, id) {
        return false;
    }
    if now_held != held {
        level.world.set_liquid(x, y, z, now_held);
    }
    let packet = update(level.world.registry(), pos, id, 0);
    to_viewers(world, layer, pos, &[packet]);
    true
}

/// The block at `pos` of a layer's world; `None` where it is not loaded or does not reach.
pub(crate) fn block_at(world: &World, layer: Entity, pos: BlockPos) -> Option<Block> {
    let level = &world.get::<Layer>(layer)?.world;
    let known = level.in_height(pos[1]) && level.is_loaded(column(pos));
    known.then(|| describe(level.registry(), level.block(pos[0], pos[1], pos[2])))
}

/// Sets a block for a handler: no event, and everyone who sees the place is shown it. A
/// container gets its block entity. Whether the cell changed; `None` where it could not be set.
pub(crate) fn put_block(world: &mut World, layer: Entity, pos: BlockPos, block: &Block) -> Option<bool> {
    let level = &world.get::<Layer>(layer)?.world;
    let id = resolve(level.registry(), block)?;
    if block_at(world, layer, pos)? == describe(level.registry(), id) {
        return Some(false);
    }
    if !set(world, layer, pos, id) {
        return None;
    }
    if let Some(name) = world.get::<Layer>(layer).and_then(|layer| layer.world.registry().get(id)).map(|state| state.name) {
        crate::containers::furnish(world, layer, pos, name, None, sapling_api::Face::Up);
    }
    Some(true)
}

/// Empties layer 1 at `pos` for everyone, if it held something.
pub(crate) fn clear_liquid(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(air) = world.get::<Layer>(layer).map(|level| level.world.registry().air_id()) else { return };
    set_liquid(world, layer, pos, air);
}

/// Writes layer 1, where a waterlogged block keeps its water, and shows it to everyone who sees
/// the place; false when it held that already or cannot be written.
pub(crate) fn set_liquid(world: &mut World, layer: Entity, pos: BlockPos, id: u32) -> bool {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return false };
    let [x, y, z] = pos;
    if !level.world.in_height(y) || level.world.liquid(x, y, z) == id || !level.world.set_liquid(x, y, z, id) {
        return false;
    }
    let packet = update(level.world.registry(), pos, id, 1);
    to_viewers(world, layer, pos, &[packet]);
    true
}

/// Shows one player both layers of each position as the world has them, undoing what its client
/// predicted there. Positions the world does not hold are skipped: the client has nothing there.
pub(crate) fn resync(world: &mut World, player: Entity, positions: &[BlockPos]) {
    let Some(layer) = world.get::<InLayer>(player).and_then(|in_layer| world.get::<Layer>(in_layer.0)) else { return };
    let level = &layer.world;
    let mut packets = Vec::with_capacity(positions.len() * 2);
    for (i, &pos) in positions.iter().enumerate() {
        let [x, y, z] = pos;
        if positions[..i].contains(&pos) || !level.in_height(y) || !level.is_loaded(column(pos)) {
            continue;
        }
        packets.push(update(level.registry(), pos, level.block(x, y, z), 0));
        packets.push(update(level.registry(), pos, level.liquid(x, y, z), 1));
        // An UpdateBlock leaves the client's block without what it held.
        packets.extend(level.block_entity(pos).map(|entity| block_entity_data(pos, entity)));
    }
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.0.extend(packets);
    }
}
