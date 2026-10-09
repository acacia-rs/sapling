//! The splash an entity makes going into water, as BDS's `Actor::doWaterSplashEffect` and its
//! in-water check make it (docs/research/splash-sound.md), and its fizz in lava.
// TODO: a rider's mount splashes louder (0.9 for 0.2); nothing rides here.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use bytes::Bytes;

use crate::body::{Body, InLiquid, Liquid, in_liquid};
use crate::components::{InLayer, Layer};
use crate::late_sounds;

/// How much of the move's speed across counts, and how loud speed makes it.
const ACROSS_WEIGHT: f32 = 0.2;
const LOUDNESS: f32 = 0.2;

/// An entity whose box was in water at its last move.
#[derive(Component)]
pub(crate) struct InWater;

/// After the entity `kind` (`id`), whose body water treats by `rule`, moved by `moved` to
/// `body`: the splash if that took it into water. BDS's volume, packed as its sounds pack one, is
/// from the move's motion before drag.
pub(crate) fn check(world: &mut World, entity: Entity, id: u64, kind: &str, moved: [f32; 3], body: &Body, rule: InLiquid) -> Option<Bytes> {
    let layer = world.get::<InLayer>(entity)?.0;
    let wet = in_liquid(&world.get::<Layer>(layer)?.world, body, rule, Liquid::Water);
    let was = world.get::<InWater>(entity).is_some();
    match (wet, was) {
        (true, false) => drop(world.entity_mut(entity).insert(InWater)),
        (false, true) => drop(world.entity_mut(entity).remove::<InWater>()),
        _ => {}
    }
    if !wet || was {
        return None;
    }
    let [dx, dy, dz] = moved;
    let volume = ((ACROSS_WEIGHT * dx * dx + dy * dy + ACROSS_WEIGHT * dz * dz).sqrt() * LOUDNESS).min(1.0);
    let [x, y, z] = body.at;
    let position = Vec3f { x, y: y + body.size[1] / 2.0, z };
    let sound = LevelSoundEvent {
        sound_id: "splash".into(),
        position,
        extra_data: (volume * 16_777_215.0) as i32,
        entity_type: kind.into(),
        is_baby_mob: false,
        is_global: false,
        entity_unique_id: id as i64,
        fire_at_position: None,
    };
    Some(sapling_net::encode(&sound))
}

/// BDS's fizz reached viewers a tick after the move it went with.
const FIZZ_LATE: u64 = 1;

/// The fizz of a body in lava (`extra_data` 0; research/captures/bds-lava.txt: an item and an
/// orb) or of a burning one put out (-1; bds-douse.txt), from the middle of its box where the tick
/// started, `start`.
pub(crate) fn fizz(world: &mut World, layer: Entity, start: &Body, extra_data: i32) {
    let [x, y, z] = start.at;
    let sound = LevelSoundEvent {
        sound_id: "fizz".into(),
        position: Vec3f { x, y: y + start.size[1] / 2.0, z },
        extra_data,
        entity_type: String::new(),
        is_baby_mob: false,
        is_global: false,
        entity_unique_id: -1,
        fire_at_position: None,
    };
    late_sounds::play_later(world, FIZZ_LATE, layer, start.at.map(|c| c.floor() as i32), sapling_net::encode(&sound));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_volume_packs_as_bds_packed_a_falling_blocks() {
        // BDS's red concrete powder: the move into the water was -0.4927161 down.
        let volume = ((0.2f32 * 0.0 + 0.4927161f32 * 0.4927161 + 0.0).sqrt() * LOUDNESS).min(1.0);
        assert_eq!((volume * 16_777_215.0) as i32, 1_653_280);
    }
}
