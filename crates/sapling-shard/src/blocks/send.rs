//! Blocks on the wire, as BDS sends them (`sapling` example `bds_block_probe`), and between the
//! world's runtime ids and the API's names.

use acacia_proto::nbt::{Nbt, Value};
use acacia_proto::packets::{BlockEntityData, LevelEvent, LevelEventEvent, LevelSoundEvent, UpdateBlock};
use acacia_proto::types::{BlockCoordinates, UpdateBlockFlags, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{Block, BlockPos, Face};
use sapling_world::{BlockEntity, BlockRegistry, ChunkPos};

use crate::components::{InLayer, Layer, Outgoing, Stage};
use crate::view::View;

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

/// The runtime id of a named state; without properties, the block's first state.
pub(crate) fn resolve(registry: &BlockRegistry, block: &Block) -> Option<u32> {
    match block.properties.is_empty() {
        true => registry.states_of(&block.name).next().map(|(id, _)| id),
        false => registry.find(&block.name, &block.properties),
    }
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

pub(crate) fn place_sound(registry: &BlockRegistry, pos: BlockPos, id: u32) -> Bytes {
    sound("place", registry, pos, id)
}

/// The sound `name` of the block `id`, which tells the client what the block is made of.
pub(crate) fn sound(name: &str, registry: &BlockRegistry, pos: BlockPos, id: u32) -> Bytes {
    sapling_net::encode(&LevelSoundEvent {
        sound_id: name.into(),
        position: centre(pos),
        extra_data: registry.network_hash(id) as i32,
        entity_type: String::new(),
        is_baby_mob: false,
        is_global: false,
        entity_unique_id: NO_ENTITY,
        fire_at_position: None,
    })
}

/// Hands `packets` to every player of `layer` whose view covers the column of `pos`, joined or
/// still loading: a client that was sent the column must hear what happens in it.
pub(crate) fn to_viewers(world: &mut World, layer: Entity, pos: BlockPos, packets: &[Bytes]) {
    let mut players = world.query::<(&InLayer, &Stage, &View, &mut Outgoing)>();
    for (in_layer, stage, view, mut out) in players.iter_mut(world) {
        if in_layer.0 == layer && *stage >= Stage::Chunks && view.covers(column(pos)) {
            out.0.extend_from_slice(packets);
        }
    }
}

/// Makes `entity` what the block at `pos` holds and shows it to those who see the block.
pub(crate) fn keep_entity(world: &mut World, layer: Entity, pos: BlockPos, entity: &Value) {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return };
    if !level.world.set_block_entity(pos, entity) {
        return tracing::warn!(?pos, "a block's entity could not be kept");
    }
    let shown: Vec<_> = level.world.block_entity(pos).map(|entity| block_entity_data(pos, entity)).into_iter().collect();
    to_viewers(world, layer, pos, &shown);
}

/// Writes a layer-0 block and shows it to everyone who sees the place. False, with nothing
/// changed, outside the world's height and where the column is not loaded or cannot be written.
pub(crate) fn set(world: &mut World, layer: Entity, pos: BlockPos, id: u32) -> bool {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return false };
    let [x, y, z] = pos;
    if !level.world.in_height(y) || !level.world.set_block(x, y, z, id) {
        return false;
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
/// container gets its block entity.
pub(crate) fn put_block(world: &mut World, layer: Entity, pos: BlockPos, block: &Block) -> bool {
    let Some(id) = world.get::<Layer>(layer).and_then(|layer| resolve(layer.world.registry(), block)) else { return false };
    let placed = set(world, layer, pos, id);
    if let Some(name) = world.get::<Layer>(layer).and_then(|layer| layer.world.registry().get(id)).map(|state| state.name).filter(|_| placed) {
        crate::containers::furnish(world, layer, pos, name, None, sapling_api::Face::Up);
    }
    placed
}

/// Empties layer 1 at `pos` for everyone, if it held something.
pub(crate) fn clear_liquid(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return };
    let ([x, y, z], air) = (pos, level.world.registry().air_id());
    if level.world.liquid(x, y, z) != air && level.world.set_liquid(x, y, z, air) {
        let packet = update(level.world.registry(), pos, air, 1);
        to_viewers(world, layer, pos, &[packet]);
    }
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
