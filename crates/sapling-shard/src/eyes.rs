//! The eye of ender thrown to find a stronghold, as BDS throws it (docs/research/end-dimension.md,
//! "The eye of ender"): `EnderEyeItem::use` 1436f6b50, `signalTo` 142f9f750, the flight
//! `EyeOfEnderPreNormalTickSystem` 14469ee30 and the end `normalTick` 142fa0450.
// TODO: BDS pushes the eye out of a player's box and turns it with its flight; sapling does neither.

use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::{MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::{GameMode, Item};
use sapling_data::{AddEntityOptions, EYE_HEIGHT};
use sapling_worldgen::mth;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Mode, NetId, Position};
use crate::inventory::{self, Inventory};
use crate::sync::Told;
use crate::{drops, entities, mobs, view};

const KIND: &str = "minecraft:eye_of_ender_signal";
const ITEM: &str = "minecraft:ender_eye";
const SIZE: [f32; 2] = [0.25, 0.25];
/// It goes after this many ticks of flight, its first the tick it was thrown.
const LIFE: u32 = 80;
/// A stronghold further off than this is flown toward, this far and this much higher.
const REACH: f32 = 12.0;
const RISE: f32 = 8.0;
/// The throw is heard this far under the eyes and this far ahead along the look (the drop point).
const SOUND_BELOW_EYES: f32 = 0.3;
const SOUND_AHEAD: f32 = 0.15;

#[derive(Component)]
pub(crate) struct Eye {
    target: [f32; 3],
    life: u32,
    survives: bool,
}

/// Where an eye at `from` flies for the stronghold block `stronghold`.
fn aim(from: [f32; 3], stronghold: [i32; 3]) -> [f32; 3] {
    let [sx, sy, sz] = stronghold.map(|c| c as f32);
    let (dx, dz) = (sx - from[0], sz - from[2]);
    let far = (dx * dx + dz * dz).sqrt();
    match far > REACH {
        true => [from[0] + dx / far * REACH, from[1] + RISE, from[2] + dz / far * REACH],
        false => [sx, sy, sz],
    }
}

/// A tick of flight: the eye moves by its motion, then turns its motion toward `target`.
pub(crate) fn step(at: [f32; 3], motion: [f32; 3], target: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let at: [f32; 3] = std::array::from_fn(|axis| at[axis] + motion[axis]);
    let mut speed = (motion[0] * motion[0] + motion[2] * motion[2]).sqrt();
    let (dx, dz) = (target[0] - at[0], target[2] - at[2]);
    let far = (dx * dx + dz * dz).sqrt();
    let angle = dz.atan2(dx);
    speed += (far - speed) * 0.0025;
    let mut rise = motion[1];
    if far < 1.0 {
        speed *= 0.8;
        rise *= 0.8;
    }
    let toward = if at[1] < target[1] { 1.0 } else { -1.0 };
    (at, [mth::cos(angle) * speed, rise + (toward - rise) * 0.015, mth::sin(angle) * speed])
}

/// An ender eye used in the air by `player`: thrown where its world locates a stronghold (only a
/// generated overworld does); true when thrown. Outside creative the eye is spent.
pub(crate) fn throw(world: &mut World, player: Entity) -> bool {
    let holds_eye = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).is_some_and(|held| held.stack.name == ITEM);
    let (Some(position), Some(layer), Some(&NetId(by))) = (world.get::<Position>(player).copied(), world.get::<InLayer>(player).map(|layer| layer.0), world.get::<NetId>(player)) else { return false };
    let eyes = [position.feet[0], position.feet[1] + EYE_HEIGHT, position.feet[2]];
    let stronghold = holds_eye.then(|| world.get::<Layer>(layer)).flatten().and_then(|layer| layer.world.locate_stronghold(eyes[0].floor() as i32, eyes[2].floor() as i32));
    let Some(stronghold) = stronghold else { return false };
    let cell = position.feet.map(|c| c.floor() as i32);
    let swing = EntityEvent { runtime_entity_id: by, event_id: EntityEventEventId::ArmSwing, data: 0, fire_at_position: None };
    view::to_viewers(world, layer, cell, &[sapling_net::encode(&swing)]);
    if world.get::<Mode>(player).is_some_and(|mode| mode.0 != GameMode::Creative) {
        inventory::change(world, player, Inventory::take_one_in_hand);
    }
    let from = eyes.map(|c| c + 0.5);
    let survives = world.get_resource_or_insert_with(Dice::default).roll(5) > 0;
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: KIND, feet: from, yaw: 0.0, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        Some(vec![sapling_net::encode(&sapling_data::add_entity(&options))])
    };
    let Some(eye) = entities::spawn(world, layer, KIND, Body::new(from, [0.0; 3], SIZE), from, announce) else { return false };
    world.entity_mut(eye).insert(Eye { target: aim(from, stronghold), life: 0, survives });
    if let Some(mut told) = world.get_mut::<Told>(eye) {
        told.tell_every_move();
    }
    let (sin, cos) = position.yaw.to_radians().sin_cos();
    let [x, y, z] = [position.feet[0] - sin * SOUND_AHEAD, eyes[1] - SOUND_BELOW_EYES, position.feet[2] + cos * SOUND_AHEAD];
    let sound = LevelSoundEvent { sound_id: "throw".into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: "minecraft:player".into(), is_baby_mob: false, is_global: false, entity_unique_id: by as i64, fire_at_position: None };
    // Held back with the eye's own packets: BDS tells the eye before the throw's sound.
    entities::tell_viewers(world, eye, layer, cell, vec![sapling_net::encode(&sound)]);
    true
}

/// Flies every eye a tick; one at the end of its life drops as an item or breaks.
pub(crate) fn tick(world: &mut World) {
    let eyes: Vec<(Entity, Entity, u64)> = world.query_filtered::<(Entity, &InLayer, &NetId), With<Eye>>().iter(world).map(|(eye, layer, id)| (eye, layer.0, id.0)).collect();
    for (eye, layer, id) in eyes {
        let (Some(body), Some(state)) = (world.get::<Body>(eye).copied(), world.get::<Eye>(eye)) else { continue };
        let (target, life, survives) = (state.target, state.life, state.survives);
        let (at, motion) = step(body.at, body.motion, target);
        if let Some(mut body) = world.get_mut::<Body>(eye) {
            (body.at, body.motion) = (at, motion);
        }
        // Its last move is not told: BDS ends it in the same tick.
        if life >= LIFE {
            end(world, eye, layer, at, survives);
            continue;
        }
        if let Some(mut state) = world.get_mut::<Eye>(eye) {
            state.life += 1;
        }
        // BDS tells the eye's viewers it stands on the ground.
        let packets = world.get_mut::<Told>(eye).map(|mut told| told.update(id, at, motion, true)).unwrap_or_default();
        entities::tell_viewers(world, eye, layer, at.map(|c| c.floor() as i32), packets);
    }
}

/// BDS takes a broken eye away the tick after its particles, a surviving one as its item appears.
fn end(world: &mut World, eye: Entity, layer: Entity, at: [f32; 3], survives: bool) {
    world.entity_mut(eye).remove::<Eye>();
    if survives {
        mobs::remove(world, eye);
        let Some(stack) = world.get::<Layer>(layer).and_then(|layer| inventory::stored(&Item::new(ITEM, 1), layer.world.registry())) else { return };
        return drops::let_go(world, layer, stack, at);
    }
    let [x, y, z] = at;
    let events = [(LevelEventEvent::ParticleEyeDespawn, 10), (LevelEventEvent::SoundItemframeRemove, 0)];
    let packets: Vec<_> = events.map(|(event, data)| sapling_net::encode(&LevelEvent { event, position: Vec3f { x, y, z }, data })).into();
    view::to_viewers(world, layer, at.map(|c| c.floor() as i32), &packets);
    entities::leave(world, eye);
}

#[cfg(test)]
mod tests {
    /// research/captures/bds-eye-throw.txt: thrown from (0.5, 151, 0.5), the stronghold at
    /// (-1454, 32, -2542); the second tick's position and where it ended, on the 81st.
    #[test]
    fn the_flight_is_bds_tick_for_tick() {
        let from = [1.0, 153.12001, 1.0];
        let target = super::aim(from, [-1454, 32, -2542]);
        let mut flight = (from, [0.0; 3]);
        let mut path = Vec::new();
        for _ in 0..=super::LIFE {
            flight = super::step(flight.0, flight.1, target);
            path.push(flight.0);
        }
        let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|axis| (a[axis] - b[axis]).abs() < 2e-5);
        assert!(near(path[1], [0.985_103_1, 153.135_01, 0.973_96]), "{:?}", path[1]);
        assert!(near(path[80], [-4.959_399_7, 161.121_86, -9.415_638]), "{:?}", path[80]);
    }
}
