//! A village's iron golems (research/captures/bds-golems.txt): only one spawned with
//! `minecraft:from_village` (its pack's `village_created` group, `behavior.defend_village_target`)
//! defends; a player's golem and a plainly summoned one never took the hitter. The first hit on a
//! grown villager had the village golem take the hitter at once; ten hits on a baby had it take none.
//! A village golem is kept so: its actor's `definitions` gain the event's group, as BDS's do.

use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::entities::{Kind, Stored};

const GOLEM: &str = "minecraft:iron_golem";
const FROM_VILLAGE: &str = "minecraft:from_village";
/// The component group `minecraft:from_village` adds (the pack's iron golem).
const VILLAGE_CREATED: &str = "minecraft:village_created";
/// How far from the hurt villager a golem defends it: its pack's `follow_range`.
const DEFENDS: f32 = 64.0;

/// An iron golem that defends its village.
#[derive(Component)]
pub(in crate::creatures) struct VillageGolem;

/// Marks a golem spawned by `minecraft:from_village`, or kept with that event's group, as its
/// village's; a new one's kept compound gains the group.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str, event: Option<&str>) {
    if kind != GOLEM {
        return;
    }
    let kept = world.get::<Stored>(entity).is_some_and(|stored| sapling_world::actor_groups(&stored.compound).iter().any(|group| group == VILLAGE_CREATED));
    if event == Some(FROM_VILLAGE) && !kept {
        if let Some(mut stored) = world.get_mut::<Stored>(entity) {
            match sapling_world::regrouped_actor(&stored.compound, &[], &[VILLAGE_CREATED]) {
                Ok(compound) => stored.compound = compound,
                Err(error) => tracing::warn!(%error, "a golem's compound is unreadable: its village is not kept"),
            }
        }
    }
    if kept || event == Some(FROM_VILLAGE) {
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
