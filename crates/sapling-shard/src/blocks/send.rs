//! Blocks on the wire, as BDS sends them (`sapling` example `bds_block_probe`), and between the
//! world's runtime ids and the API's names.

use acacia_proto::nbt::{Nbt, Value};
pub(crate) use acacia_proto::packets::UpdateBlockSyncedTransitionType as Transition;
use acacia_proto::packets::{BlockEntityData, LevelEvent, LevelEventEvent, LevelSoundEvent, UpdateBlock, UpdateBlockSynced};
use acacia_proto::types::{BlockCoordinates, UpdateBlockFlags, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{Block, BlockPos, Face};
use sapling_world::{BlockEntity, BlockRegistry, ChunkPos};

mod sections;

pub(crate) use sections::{batched, hold, set_many, set_sectioned, show_held, show_states, show_together};

use crate::components::{InLayer, Layer, Outgoing};

/// BDS sets these two on every UpdateBlock it sends.
pub(crate) const FLAGS: UpdateBlockFlags = UpdateBlockFlags(UpdateBlockFlags::NEIGHBORS.0 | UpdateBlockFlags::NETWORK.0);
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

/// Whether plugins let fire or a liquid at `source` put the state `id` names into `pos`
/// ([`sapling_api::BlockSpreadEvent`]); `id` is only asked where one listens.
pub(crate) fn may_spread(world: &mut World, layer: Entity, pos: BlockPos, source: BlockPos, id: impl FnOnce(&BlockRegistry) -> Option<u32>) -> bool {
    !crate::plugins::stopped_in(world, layer, |world| {
        let block = super::level(world, layer).and_then(|level| id(level.registry()).map(|id| describe(level.registry(), id)));
        sapling_api::BlockSpreadEvent::new(pos, source, block.unwrap_or_else(Block::air))
    })
}

/// The runtime id of a named state: the block's default state (`sapling_data::default_properties`)
/// with the properties it names set, so that `water` with `liquid_depth=3` needs no other.
pub(crate) fn resolve(registry: &BlockRegistry, block: &Block) -> Option<u32> {
    let first = registry.states_of(&block.name).next().map(|(id, _)| id)?;
    let default = sapling_data::default_properties(&block.name).and_then(|properties| registry.find(&block.name, properties)).unwrap_or(first);
    if block.properties.is_empty() {
        return Some(default);
    }
    registry.find(&block.name, &block.properties).or_else(|| {
        let mut pairs = block.properties.split(',').map(|pair| pair.split_once('='));
        pairs.try_fold(default, |id, pair| pair.and_then(|(key, value)| super::with_property(registry, id, key, value)))
    })
}

fn update(registry: &BlockRegistry, pos: BlockPos, id: u32, layer: u32) -> Bytes {
    update_flagged(registry, pos, id, layer, FLAGS)
}

fn update_flagged(registry: &BlockRegistry, [x, y, z]: BlockPos, id: u32, layer: u32, flags: UpdateBlockFlags) -> Bytes {
    sapling_net::encode(&UpdateBlock { position: BlockCoordinates { x, y, z }, block_runtime_id: registry.network_hash(id), flags, layer })
}

/// What a block holds, as clients are shown it.
pub(crate) fn block_entity_data([x, y, z]: BlockPos, entity: &BlockEntity) -> Bytes {
    sapling_net::encode(&BlockEntityData { position: BlockCoordinates { x, y, z }, nbt: Nbt { name: String::new(), value: entity.shown() }.into() })
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
    let Some(id) = write(world, layer, pos, id) else { return false };
    if sections::held(world, layer, pos) {
        return true;
    }
    let Some(packet) = world.get::<Layer>(layer).map(|level| update(level.world.registry(), pos, id, 0)) else { return false };
    to_viewers(world, layer, pos, &[packet]);
    true
}

/// [`set`] told with the network flag alone, as BDS told a bed's `occupied_bit`
/// (research/captures/bds-wake.txt).
pub(crate) fn set_network_only(world: &mut World, layer: Entity, pos: BlockPos, id: u32) -> bool {
    let Some(id) = write(world, layer, pos, id) else { return false };
    let Some(packet) = world.get::<Layer>(layer).map(|level| update_flagged(level.world.registry(), pos, id, 0, UpdateBlockFlags::NETWORK)) else { return false };
    to_viewers(world, layer, pos, &[packet]);
    true
}

/// [`set`] without telling anyone; [`tell_synced`] does, once the entity it was for exists.
pub(crate) fn set_quietly(world: &mut World, layer: Entity, pos: BlockPos, id: u32) -> bool {
    write(world, layer, pos, id).is_some()
}

/// Writes layer 0 as [`set`] does: the state written, or `None` when nothing changed.
fn write(world: &mut World, layer: Entity, pos: BlockPos, id: u32) -> Option<u32> {
    let mut level = world.get_mut::<Layer>(layer)?;
    let [x, y, z] = pos;
    if !level.world.in_height(y) || !level.world.is_loaded(column(pos)) {
        return None;
    }
    let (old, held) = (level.world.block(x, y, z), level.world.liquid(x, y, z));
    let (id, now_held) = super::waterlog::written(level.world.registry(), id, old, held);
    if (id, now_held) == (old, held) || !level.world.set_block(x, y, z, id) {
        return None;
    }
    if now_held != held {
        level.world.set_liquid(x, y, z, now_held);
    }
    let name = |id: u32| level.world.registry().get(id).map_or("", |state| state.name);
    let (was, now) = (name(old), name(id));
    super::tripwire::written(world, layer, pos, was, now);
    super::sculk::written(world, layer, pos, was, now);
    super::redstone::notice(world, layer, pos);
    super::daylight::written(world, layer, pos);
    super::fire::written(world, layer, pos);
    super::bubbles::written(world, layer, pos);
    super::coral::written(world, layer, pos);
    super::sponge::written(world, layer, pos);
    super::ores::written(world, layer, pos);
    super::growth::written(world, layer, pos);
    super::leaves::written(world, layer, pos, old);
    super::scaffolds::written(world, layer, pos);
    super::portals::written(world, layer, pos);
    Some(id)
}

/// Shows everyone who sees `pos` its layer-0 block again, unchanged.
pub(crate) fn show(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let packet = update(level.registry(), pos, level.block(pos[0], pos[1], pos[2]), 0);
    to_viewers(world, layer, pos, &[packet]);
}

/// [`show`] with what the block holds after it, as BDS showed a barrel turned
/// (research/captures/bds-shut.txt).
pub(crate) fn show_whole(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let mut packets = vec![update(level.registry(), pos, level.block(pos[0], pos[1], pos[2]), 0)];
    packets.extend(level.block_entity(pos).map(|entity| block_entity_data(pos, entity)));
    to_viewers(world, layer, pos, &packets);
}

/// Shows everyone who sees `pos` its block as the entity `entity` took it (`Create`: a falling
/// block leaving it) or left it (`Destroy`: one landing), as BDS does in UpdateBlockSynced.
pub(crate) fn tell_synced(world: &mut World, layer: Entity, pos: BlockPos, entity: u64, transition: Transition) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let [x, y, z] = pos;
    let block_runtime_id = level.registry().network_hash(level.block(x, y, z));
    let packet = UpdateBlockSynced { position: BlockCoordinates { x, y, z }, block_runtime_id, flags: FLAGS, layer: 0, entity_unique_id: entity, transition_type: transition };
    let packet = sapling_net::encode(&packet);
    to_viewers(world, layer, pos, &[packet]);
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
    let before = level.block_entity(pos).map(|entity| entity.nbt.clone());
    if !set(world, layer, pos, id) {
        return None;
    }
    if let Some(name) = world.get::<Layer>(layer).and_then(|layer| layer.world.registry().get(id)).map(|state| state.name) {
        crate::containers::furnish(world, layer, pos, name, None, sapling_api::Face::Up);
        super::pistons::furnish(world, layer, pos);
        super::command_blocks::furnish(world, layer, pos, before.as_ref());
        super::structure_blocks::furnish(world, layer, pos, before.as_ref());
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

/// Shows one player layer 0 of each of `positions` in order, repeats and all, as BDS showed its
/// placer the cells about a place (research/captures/bds-place-faces.txt).
pub(crate) fn show_to(world: &mut World, player: Entity, positions: &[BlockPos]) {
    let Some(layer) = world.get::<InLayer>(player).and_then(|in_layer| world.get::<Layer>(in_layer.0)) else { return };
    let level = &layer.world;
    let shown = positions.iter().filter(|pos| level.in_height(pos[1]) && level.is_loaded(column(**pos)));
    let packets: Vec<Bytes> = shown.map(|&[x, y, z]| update(level.registry(), [x, y, z], level.block(x, y, z), 0)).collect();
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.0.extend(packets);
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
