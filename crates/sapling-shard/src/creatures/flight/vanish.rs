//! A player's arrow or trident is heard `disappeared` a block above where it starts as it
//! appears, and `reappeared` from its middle two ticks on, before that tick's move
//! (research/captures/bds-r1shots.txt). A thing it threw is heard so too, from its middle (a wind
//! charge 0.9 up) (bds-small.txt, bds-bg2player.txt, bds-q3wind.txt).

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use super::Arrow;
use crate::body::Body;
use crate::components::NetId;
use crate::creatures::Lands;

const SHOT_VANISHED_ABOVE: f32 = 1.0;
const WIND_VANISHED_ABOVE: f32 = 0.9;
const REAPPEARS_AT_AGE: u64 = 2;

#[derive(Component)]
struct Vanished;

/// Marks `shot` as one that reappears, and gives its `disappeared` sound.
pub(crate) fn vanish(world: &mut World, shot: Entity) -> Option<bytes::Bytes> {
    let (body, id, shot_kind) = (world.get::<Body>(shot)?, world.get::<NetId>(shot)?.0, world.get::<Arrow>(shot)?.shot);
    let above = match shot_kind.lands {
        Lands::Hurts => SHOT_VANISHED_ABOVE,
        Lands::Wind => WIND_VANISHED_ABOVE,
        _ => body.size[1] / 2.0,
    };
    let at = body.at;
    world.entity_mut(shot).insert(Vanished);
    Some(sound("disappeared", shot_kind.kind, id, [at[0], at[1] + above, at[2]]))
}

/// Tells a vanished shot's `reappeared` in the tick it is due, from the middle of `body`.
pub(in crate::creatures) fn reappear(world: &mut World, shot: Entity, layer: Entity, id: u64, age: u64, body: &Body) {
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
