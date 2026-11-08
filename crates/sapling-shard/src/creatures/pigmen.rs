//! A zombie pigman's anger, by its pack's `minecraft:angry` (`broadcast_anger`, `broadcast_range`
//! 20): one that takes a player for its prey sets every pigman within 20 on that player. BDS's two
//! bystanders 9 off took the player as the one hurt did (research/captures/bds-h2dmgm.txt).
// BDS's three took the player a tick after the hurt; here the hurt one and those near take it in its tick.

use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::entities::Kind;

const KIND: &str = "minecraft:zombie_pigman";
const SPREADS: f32 = 20.0;

/// The mob at `at` took `player` for its prey: if a pigman, so does every pigman within 20.
pub(super) fn spread_anger(world: &mut World, pigman: Entity, layer: Entity, at: [f32; 3], player: Entity) {
    if world.get::<Kind>(pigman).is_none_or(|kind| kind.0 != KIND) {
        return;
    }
    let near: Vec<(Entity, u64, Body)> = world
        .query::<(Entity, &InLayer, &NetId, &Body, &Kind)>()
        .iter(world)
        .filter(|(other, in_layer, _, body, kind)| *other != pigman && in_layer.0 == layer && kind.0 == KIND && (0..3).map(|axis| (body.at[axis] - at[axis]).powi(2)).sum::<f32>() <= SPREADS * SPREADS)
        .map(|(other, _, id, body, _)| (other, id.0, *body))
        .collect();
    for (other, id, body) in near {
        if super::is_alive(world, other) {
            super::hunting::provoke(world, other, layer, id, KIND, &body, player);
        }
    }
}
