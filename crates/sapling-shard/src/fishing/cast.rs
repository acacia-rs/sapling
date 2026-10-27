//! A cast, as BDS 1.26 casts (research/captures/bds-fish.txt): the hook leaves the fisher's drop
//! point, 0.3 under its eyes and 0.15 ahead, moved by the fisher's own motion, at 1.3 along the
//! look with no spread. The throw is heard from the drop point.
// TODO: only a standing fisher was captured; its motion is BDS's resting fall.

use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1};
use bevy_ecs::prelude::*;
use sapling_data::{AddEntityOptions, EYE_HEIGHT};

use super::{HOOK, Hook, Line};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Tick};
use crate::creatures::facing;
use crate::dragon::cloud::item;
use crate::entities;

const SIZE: [f32; 2] = [0.15, 0.15];
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
        let options = AddEntityOptions { id, identifier: HOOK, feet: from, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        let mut shown = sapling_data::add_entity(&options);
        shown.velocity = acacia_proto::types::Vec3f { x: motion[0], y: motion[1], z: motion[2] };
        shown.metadata.retain(|kept| kept.key != Key::OwnerEid);
        shown.metadata.push(item(Key::OwnerEid, Plain::Long(owner as i64)));
        Some(vec![sapling_net::encode(&shown)])
    };
    let Some(hook) = entities::spawn(world, layer, HOOK, Body::new(from, motion, SIZE), from, announce) else { return };
    let cast = world.resource::<Tick>().0;
    world.entity_mut(hook).insert(Hook { fisher: player, cast, bob: 0.0, angle: 0.0, lured: 0, hooked: 0, nibble: 0, in_ground: false, target: None, jungle });
    world.entity_mut(player).insert(Line(hook));
    let sound = crate::creatures::sound("throw", drop, "minecraft:player");
    crate::view::to_viewers(world, layer, feet.map(|c| c.floor() as i32), &[sound]);
}
