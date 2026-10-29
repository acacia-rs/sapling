//! A village's iron golems (research/captures/bds-golems.txt): only one spawned with
//! `minecraft:from_village` (its pack's `village_created` group, `behavior.defend_village_target`)
//! defends; a player's golem and a plainly summoned one never took the hitter. The first hit on a
//! grown villager had the village golem take the hitter at once; ten hits on a baby had it take none.
// TODO: whether a golem is the village's is not kept with the world (BDS keeps its component groups).

use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::entities::Kind;

const GOLEM: &str = "minecraft:iron_golem";
const FROM_VILLAGE: &str = "minecraft:from_village";
/// How far from the hurt villager a golem defends it: its pack's `follow_range`.
const DEFENDS: f32 = 64.0;

/// An iron golem that defends its village.
#[derive(Component)]
pub(in crate::creatures) struct VillageGolem;

/// Marks a golem spawned by `minecraft:from_village` as its village's.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str, event: Option<&str>) {
    if kind == GOLEM && event == Some(FROM_VILLAGE) {
        world.entity_mut(entity).insert(VillageGolem);
    }
}

/// The village golems near the grown villager at `at` in `layer` take `player`, who hurt it.
pub(super) fn defend(world: &mut World, player: Entity, layer: Entity, at: [f32; 3]) {
    let near = |body: &Body| (0..3).map(|axis| (body.at[axis] - at[axis]).powi(2)).sum::<f32>() <= DEFENDS * DEFENDS;
    let golems: Vec<(Entity, u64, String, Body)> = world
        .query_filtered::<(Entity, &NetId, &Kind, &Body, &InLayer), With<VillageGolem>>()
        .iter(world)
        .filter(|(.., body, on)| on.0 == layer && near(body))
        .map(|(golem, id, kind, body, _)| (golem, id.0, kind.0.clone(), *body))
        .collect();
    for (golem, id, kind, body) in golems {
        super::super::hunting::provoke(world, golem, layer, id, &kind, &body, player);
    }
}
