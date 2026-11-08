//! The particles and sounds blocks make, as BDS sends them.

use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use bytes::Bytes;
use sapling_api::{BlockPos, Face};
use sapling_world::BlockRegistry;

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

fn centre(pos: BlockPos) -> Vec3f {
    let [x, y, z] = pos.map(|c| c as f32 + 0.5);
    Vec3f { x, y, z }
}

/// The burst of fragments when a block of `id` breaks.
pub(crate) fn destroy_particles(registry: &BlockRegistry, pos: BlockPos, id: u32) -> Bytes {
    sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleDestroy, position: centre(pos), data: sapling_world::block_hash(registry, id) as i32 })
}

/// The few fragments a hit knocks off `face`.
pub(crate) fn punch_particles(registry: &BlockRegistry, pos: BlockPos, face: Face, id: u32) -> Bytes {
    sapling_net::encode(&LevelEvent { event: PUNCH[face as usize], position: centre(pos), data: sapling_world::block_hash(registry, id) as i32 })
}

/// The sound of the block `id` placed: BDS names the block's default state, not the one placed (a
/// chest's faces north, research/captures/bds-u3chest.txt).
pub(crate) fn place_sound(registry: &BlockRegistry, pos: BlockPos, id: u32) -> Bytes {
    let Some(name) = registry.get(id).map(|state| state.name) else { return sound("place", registry, pos, id) };
    let first = registry.states_of(name).next().map_or(id, |(first, _)| first);
    let default = sapling_data::default_properties(name).and_then(|properties| registry.find(name, properties)).unwrap_or(first);
    sound("place", registry, pos, default)
}

/// The sound `name` of the block `id`, which tells the client what the block is made of.
pub(crate) fn sound(name: &str, registry: &BlockRegistry, pos: BlockPos, id: u32) -> Bytes {
    sound_at(name, registry, centre(pos), id)
}

/// [`sound`] from a point other than the block's centre.
pub(crate) fn sound_at(name: &str, registry: &BlockRegistry, position: Vec3f, id: u32) -> Bytes {
    noise(name, position, sapling_world::block_hash(registry, id) as i32)
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
