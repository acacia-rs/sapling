//! A charging wither's spin and random looks, as BDS's (`WitherBoss` 0xbbba9f0 on Linux, under gdb on
//! ab1wither2, docs/research/wither-lightning-fireworks.md): each tick of its charge it turns its body
//! and head the other way round by a speed that starts at 10 degrees and grows 0.2 a tick, and each time
//! its yaw comes round past 0 from below it plays its `spawn` sound, global, at its feet. Its
//! `random_look_around` (`RandomLookAroundGoal`, start 0x5760e30: -180 to 180 degrees about the body,
//! 20 to 39 ticks, a 0.02 chance a tick) holds the head on a yaw while the body spins under it, no more
//! than [`HEAD_MOST`] off the body (bds-ah1wither-gdb2.txt: held, then dragged at 75 off).

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use super::{KIND, Wither};
use crate::blocks::Dice;
use crate::body::Body;
use crate::creatures::legs;
use crate::entities;
use crate::sync::Told;

const SPEED: f32 = 10.0;
const FASTER: f32 = 0.2;
const CHANCE: f32 = 0.02;
const LOOK_TICKS: [u32; 2] = [20, 20];
const HEAD_MOST: f32 = 75.0;

pub(super) struct Spin {
    speed: f32,
    /// The yaw its random look holds the head to, and the ticks left of it.
    look: Option<(f32, u32)>,
}

impl Default for Spin {
    fn default() -> Self {
        Spin { speed: SPEED, look: None }
    }
}

fn wrap(degrees: f32) -> f32 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

/// A tick of its spin while it charges, its head on its random look if it takes one.
pub(super) fn turn(world: &mut World, wither: Entity, layer: Entity, body: &Body) {
    let Some(mut spin) = world.get_mut::<Wither>(wither).map(|mut kept| std::mem::take(&mut kept.spin)) else { return };
    let was = world.get::<legs::Legs>(wither).map_or(0.0, |legs| legs.yaw);
    spin.speed += FASTER;
    let yaw = wrap(was - spin.speed);
    if spin.look.is_none() {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        if dice.fraction() < CHANCE {
            spin.look = Some((yaw + dice.fraction() * 360.0 - 180.0, LOOK_TICKS[0] + dice.roll(LOOK_TICKS[1])));
        }
    }
    let head = spin.look.map_or(yaw, |(held, _)| yaw + wrap(held - yaw).clamp(-HEAD_MOST, HEAD_MOST));
    spin.look = spin.look.and_then(|(held, left)| left.checked_sub(1).map(|left| (held, left)));
    if let Some(mut legs) = world.get_mut::<legs::Legs>(wither) {
        legs.yaw = yaw;
    }
    if let Some(mut told) = world.get_mut::<Told>(wither) {
        told.face(yaw, head);
    }
    if let Some(mut kept) = world.get_mut::<Wither>(wither) {
        kept.spin = spin;
    }
    if was < 0.0 && yaw >= 0.0 {
        let [x, y, z] = body.at;
        let sound = LevelSoundEvent { sound_id: "spawn".into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: KIND.into(), is_baby_mob: false, is_global: true, entity_unique_id: -1, fire_at_position: None };
        entities::tell_viewers(world, wither, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&sound)]);
    }
}
