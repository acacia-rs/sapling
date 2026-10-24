//! A dragon's death, as BDS's went (research/captures/bds-dragon-fight.txt, bds-dragon-again.txt):
//! it stays where it died, its health told as gone, its flags without `MOVING`, its death heard
//! two above it; from the 155th tick every fifth lets out 8% of its experience, the 200th 20%
//! more, and then it is gone and the fight goes on (`dragon_gone`). The first dragon of an End is
//! worth 12000, the others 500.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::creatures::Health;
use crate::{entities, mobs, orbs};

const FIRST: i32 = 12_000;
const LATER: i32 = 500;
const LASTS: u64 = 200;
/// Experience comes every [`EVERY`] ticks after [`FROM`], a share of it each time, and the end's
/// share on top at the last.
const FROM: u64 = 150;
const EVERY: u64 = 5;
const SHARE: f32 = 0.08;
const LAST_SHARE: f32 = 0.2;
/// The tick after its death its flags are told again, and the one it is heard dying, from this
/// far above its feet.
const FLAGS_AFTER: u64 = 1;
const HEARD_AFTER: u64 = 2;
const HEARD_ABOVE: f32 = 2.0;

/// A dragon dying since the tick it holds, worth `worth`.
#[derive(Component)]
pub(super) struct Dying {
    since: u64,
    worth: i32,
}

/// The dragon begins to die, the first of its End or not.
pub(super) fn begin(world: &mut World, dragon: Entity, first: bool) {
    let since = world.resource::<Tick>().0;
    world.entity_mut(dragon).insert(Dying { since, worth: if first { FIRST } else { LATER } });
    let Some(max) = world.get_mut::<Health>(dragon).map(|mut health| {
        health.end();
        health.max()
    }) else { return };
    let (Some(id), Some(layer), Some(body)) = (world.get::<NetId>(dragon).map(|id| id.0), world.get::<InLayer>(dragon).map(|layer| layer.0), world.get::<Body>(dragon).copied()) else { return };
    let gone = sapling_net::encode(&sapling_data::mob_health_update(id, 0.0, max));
    entities::tell_viewers(world, dragon, layer, body.at.map(|c| c.floor() as i32), vec![gone]);
}

pub(super) fn is_dying(world: &World, dragon: Entity) -> bool {
    world.get::<Dying>(dragon).is_some()
}

/// A tick of each dying dragon of `layer`.
pub(super) fn tick(world: &mut World, layer: Entity) {
    let now = world.resource::<Tick>().0;
    let dying: Vec<(Entity, u64, i32, Body)> = world
        .query::<(Entity, &Dying, &InLayer, &Body)>()
        .iter(world)
        .filter(|(_, _, in_layer, _)| in_layer.0 == layer)
        .map(|(dragon, dying, _, body)| (dragon, now - dying.since, dying.worth, *body))
        .collect();
    for (dragon, ticks, worth, body) in dying {
        let id = world.get::<NetId>(dragon).map_or(0, |id| id.0);
        let cell = body.at.map(|c| c.floor() as i32);
        if ticks == FLAGS_AFTER {
            entities::tell_viewers(world, dragon, layer, cell, vec![sapling_net::encode(&sapling_data::mob_flags_set(id, sapling_data::grown_flags(super::DRAGON, acacia_proto::types::MetadataFlags1(0))))]);
        }
        if ticks == HEARD_AFTER {
            let [x, y, z] = body.at;
            let sound = LevelSoundEvent { sound_id: "death".into(), position: Vec3f { x, y: y + HEARD_ABOVE, z }, extra_data: -1, entity_type: super::DRAGON.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
            entities::tell_viewers(world, dragon, layer, cell, vec![sapling_net::encode(&sound)]);
        }
        if ticks > FROM && ticks % EVERY == 0 {
            orbs::spawn(world, layer, body.at, (worth as f32 * SHARE) as i32);
        }
        if ticks == LASTS {
            orbs::spawn(world, layer, body.at, (worth as f32 * LAST_SHARE) as i32);
            mobs::remove(world, dragon);
            super::dragon_gone(world, layer);
        }
    }
}
