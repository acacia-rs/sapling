//! A player's arrow or trident is heard `disappeared` a block above where it starts as it
//! appears, and `reappeared` from its middle two ticks on, before that tick's move
//! (research/captures/bds-r1shots.txt).

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use super::Arrow;
use crate::body::Body;
use crate::components::NetId;

const VANISHED_ABOVE: f32 = 1.0;
const REAPPEARS_AT_AGE: u64 = 2;

#[derive(Component)]
struct Vanished;

/// Marks `shot` as one that reappears, and gives its `disappeared` sound.
pub(crate) fn vanish(world: &mut World, shot: Entity) -> Option<bytes::Bytes> {
    let (at, id, kind) = (world.get::<Body>(shot)?.at, world.get::<NetId>(shot)?.0, world.get::<Arrow>(shot)?.shot.kind);
    world.entity_mut(shot).insert(Vanished);
    Some(sound("disappeared", kind, id, [at[0], at[1] + VANISHED_ABOVE, at[2]]))
}

/// Tells a vanished shot's `reappeared` in the tick it is due, from the middle of `body`.
pub(super) fn reappear(world: &mut World, shot: Entity, layer: Entity, id: u64, age: u64, body: &Body) {
    if age != REAPPEARS_AT_AGE || world.entity_mut(shot).take::<Vanished>().is_none() {
        return;
    }
    let Some(kind) = world.get::<Arrow>(shot).map(|arrow| arrow.shot.kind) else { return };
    let [x, y, z] = body.at;
    let heard = sound("reappeared", kind, id, [x, y + body.size[1] / 2.0, z]);
    crate::view::to_viewers(world, layer, body.at.map(|c| c.floor() as i32), &[heard]);
}

fn sound(name: &str, kind: &str, id: u64, [x, y, z]: [f32; 3]) -> bytes::Bytes {
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: kind.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None })
}
