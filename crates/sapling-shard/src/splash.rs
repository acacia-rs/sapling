//! The splash an entity makes going into water, as BDS's `Actor::doWaterSplashEffect` and its
//! in-water check make it (docs/research/splash-sound.md).
// TODO: a rider's mount splashes louder (0.9 for 0.2); nothing rides here.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::World as Level;

use crate::body::Body;
use crate::components::Layer;

/// The in-water check leaves out this much of the box at the top and at the bottom, and a hair
/// at the sides.
const SHRINK_UP_DOWN: f32 = 0.4;
const SHRINK_ACROSS: f32 = 0.001;
/// How much of the move's speed across counts, and how loud speed makes it.
const ACROSS_WEIGHT: f32 = 0.2;
const LOUDNESS: f32 = 0.2;

/// An entity whose box was in water at its last move.
#[derive(Component)]
pub(crate) struct InWater;

fn in_water(level: &Level, body: &Body) -> bool {
    let ([x, y, z], [width, height]) = (body.at, body.size);
    let half = width / 2.0 - SHRINK_ACROSS;
    let (low, high) = ([x - half, y + SHRINK_UP_DOWN, z - half], [x + half, y + height - SHRINK_UP_DOWN, z + half]);
    if (0..3).any(|axis| high[axis] < low[axis]) {
        return false;
    }
    let cells = |axis: usize| low[axis].floor() as i32..=high[axis].floor() as i32;
    let is_water = |id: u32| level.registry().get(id).is_some_and(|state| state.name.ends_with("water"));
    cells(0).any(|cx| cells(1).any(|cy| cells(2).any(|cz| level.in_height(cy) && (is_water(level.block(cx, cy, cz)) || is_water(level.liquid(cx, cy, cz))))))
}

/// After the entity `kind` (`id`) moved by `moved` to `body`: the splash if that took it into
/// water. BDS's volume, packed as its sounds pack one, is from the move's motion before drag.
pub(crate) fn check(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, moved: [f32; 3], body: &Body) -> Option<Bytes> {
    let wet = in_water(&world.get::<Layer>(layer)?.world, body);
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
