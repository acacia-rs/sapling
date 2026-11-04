//! Piglins and piglin brutes out of the Nether become zombified piglins, by their packs'
//! `zombification_sensor`: out of it, they shake (`is_shaking`, flag `VIBRATING`) and a 15 s timer
//! runs; back in, it stops. BDS's summoned piglin shook a tick after it came and turned 303 ticks
//! after, holding its sword, heard `converted_to_zombified` (research/captures/bds-t2barter.txt).

use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;
use sapling_api::SpawnReason;

use super::Creature;
use super::turning::Turning;
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::entities::{self, Kind};

const KINDS: [&str; 2] = ["minecraft:piglin", "minecraft:piglin_brute"];
const INTO: &str = "minecraft:zombie_pigman";
const SOUND: &str = "converted_to_zombified";
const NETHER: i32 = 1;
/// From the first tick a creature tick sees it out of the Nether, a tick after it came.
const TURNS: u64 = 302;

/// A piglin out of the Nether, shaking.
#[derive(Component)]
pub(super) struct Zombifying;

fn shake(world: &mut World, piglin: Entity, layer: Entity, kind: &str, on: bool) {
    let Some(id) = world.get::<NetId>(piglin).map(|id| id.0) else { return };
    let flags = entities::mob_flags(world, piglin, kind, MetadataFlags1::VIBRATING, on);
    let Some(at) = world.get::<crate::body::Body>(piglin).map(|body| body.at.map(|c| c.floor() as i32)) else { return };
    entities::tell_viewers(world, piglin, layer, at, vec![sapling_net::encode(&sapling_data::mob_flags_set(id, flags))]);
}

/// A tick: piglins out of the Nether start zombifying, those back in it stop.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let piglins: Vec<(Entity, Entity, String)> = world
        .query_filtered::<(Entity, &InLayer, &Kind), With<Creature>>()
        .iter(world)
        .filter(|(_, _, kind)| KINDS.contains(&kind.0.as_str()))
        .map(|(piglin, layer, kind)| (piglin, layer.0, kind.0.clone()))
        .collect();
    for (piglin, layer, kind) in piglins {
        if super::is_dying(world, piglin) {
            continue;
        }
        let out = world.get::<Layer>(layer).is_some_and(|layer| layer.world.dimension() != NETHER);
        match (out, world.get::<Zombifying>(piglin).is_some()) {
            (true, false) => {
                shake(world, piglin, layer, &kind, true);
                world.entity_mut(piglin).insert((Zombifying, Turning { into: INTO, sound: Some(SOUND), at: now + TURNS, keeps_gear: true, reason: SpawnReason::Transformed }));
            }
            (false, true) => {
                world.entity_mut(piglin).remove::<(Zombifying, Turning)>();
                shake(world, piglin, layer, &kind, false);
            }
            _ => {}
        }
    }
}
