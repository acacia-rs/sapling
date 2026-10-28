//! A cast, as BDS 1.26 casts (research/captures/bds-fish.txt): the hook leaves the fisher's drop
//! point, 0.3 under its eyes and 0.15 ahead, moved by the fisher's own motion, at 1.3 along the
//! look with no spread. The throw is heard from the drop point. The hook comes invisible, with its
//! `disappeared` sound 0.9 above it; the next tick it shows, moving, and the one after it is heard
//! `reappeared` from its middle and stops showing it moves.
// TODO: only a standing fisher was captured; its motion is BDS's resting fall.

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use bevy_ecs::prelude::*;
use sapling_data::{AddEntityOptions, EYE_HEIGHT};

use super::{HOOK, Hook, Line};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Tick};
use crate::creatures::facing;
use crate::dragon::cloud::{item, with};
use crate::entities;

const SIZE: [f32; 2] = [0.15, 0.15];
/// Its flags as cast, the tick after and from the second tick on (bds-fish.txt).
const CAST_FLAGS: i64 = Flags::INVISIBLE.0 | Flags::HAS_COLLISION.0 | Flags::AFFECTED_BY_GRAVITY.0;
const SHOWN_FLAGS: i64 = Flags::CAN_SHOW_NAMETAG.0 | Flags::HAS_COLLISION.0 | Flags::AFFECTED_BY_GRAVITY.0;
const VANISHED_ABOVE: f32 = 0.9;
const POWER: f32 = 1.3;
const BELOW_EYES: f32 = 0.3;
const AHEAD: f32 = 0.15;
/// A standing player's motion, which BDS adds to where the hook starts.
const RESTING_FALL: [f32; 3] = [0.0, -0.0784, 0.0];

pub(super) fn cast(world: &mut World, player: Entity) {
    let (Some(layer), Some(position), Some(owner)) = (world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).copied(), world.get::<NetId>(player).map(|id| id.0)) else { return };
    let (yaw, pitch) = (position.yaw.to_radians(), position.pitch.to_radians());
    let feet = position.feet;
    let drop = [feet[0] - yaw.sin() * AHEAD, feet[1] + EYE_HEIGHT - BELOW_EYES, feet[2] + yaw.cos() * AHEAD];
    let from = [0, 1, 2].map(|axis| drop[axis] + RESTING_FALL[axis]);
    let motion = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()].map(|c| c * POWER);
    let jungle = world.get::<Layer>(layer).and_then(|level| level.world.biome(from[0].floor() as i32, from[1].floor() as i32, from[2].floor() as i32)).is_some_and(|biome| sapling_data::biome_has_tag(biome, "jungle"));
    // A hook's yaw turns the other way from an arrow's: BDS shows one cast east at 90.
    let [yaw, pitch] = facing(motion);
    let yaw = -yaw;
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: HOOK, feet: from, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags: Flags(0), health: None, name: "" };
        let mut shown = sapling_data::add_entity(&options);
        shown.velocity = acacia_proto::types::Vec3f { x: motion[0], y: motion[1], z: motion[2] };
        with(&mut shown.metadata, CAST_FLAGS as u64, vec![item(Key::OwnerEid, Plain::Long(owner as i64))]);
        Some(vec![sapling_net::encode(&shown)])
    };
    let Some(hook) = entities::spawn(world, layer, HOOK, Body::new(from, motion, SIZE), from, announce) else { return };
    let cast = world.resource::<Tick>().0;
    world.entity_mut(hook).insert(Hook { fisher: player, cast, bob: 0.0, angle: 0.0, lured: 0, hooked: 0, nibble: 0, in_ground: false, target: None, jungle });
    world.entity_mut(player).insert(Line(hook));
    let id = world.get::<NetId>(hook).map_or(0, |id| id.0);
    let vanished = sound("disappeared", [from[0], from[1] + VANISHED_ABOVE, from[2]], id);
    let thrown = crate::creatures::sound("throw", drop, "minecraft:player");
    crate::view::to_viewers(world, layer, feet.map(|c| c.floor() as i32), &[vanished, thrown]);
}

fn sound(name: &str, [x, y, z]: [f32; 3], id: u64) -> bytes::Bytes {
    let sound = acacia_proto::packets::LevelSoundEvent { sound_id: name.into(), position: acacia_proto::types::Vec3f { x, y, z }, extra_data: -1, entity_type: HOOK.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
    sapling_net::encode(&sound)
}

/// What a hook `age` ticks out shows of its coming: its flags, and the `reappeared` sound.
pub(super) fn appearing(id: u64, age: u64, at: [f32; 3]) -> Vec<bytes::Bytes> {
    let flags = |flags: i64| {
        let mut metadata = vec![item(Key::Flags, Plain::Long(0))];
        with(&mut metadata, flags as u64, Vec::new());
        sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
    };
    match age {
        1 => vec![flags(SHOWN_FLAGS | Flags::MOVING.0)],
        2 => vec![sound("reappeared", [at[0], at[1] + SIZE[1] / 2.0, at[2]], id), flags(SHOWN_FLAGS)],
        _ => Vec::new(),
    }
}
