//! The dragon's boss bar (its `minecraft:boss` component, research/dragon-fight-decompile.md 2), as
//! BDS showed it (research/captures/bds-dragon-watch.txt): a ShowBar named
//! `entity.ender_dragon.name`, purple, its health's share, every second to the players of its
//! End within 125 of it; a dying dragon's at 0. It goes with the dragon.

use acacia_proto::packets::{BossEvent, BossEventColor, BossEventOverlay, BossEventType};
use bevy_ecs::prelude::*;

use super::Dragon;
use crate::body::Body;
use crate::components::{InLayer, NetId, Outgoing, Position, Tick, in_world};
use crate::creatures::Health;

const TITLE: &str = "entity.ender_dragon.name";
const REACH: f32 = 125.0;
/// Ticks between two showings (BDS's came 1000 to 1050 ms apart, by its clock), and from the
/// dragon's coming to the first.
const EVERY: u64 = 20;
const FIRST: u64 = 2;

/// Shows the bar of each dragon of `layer` whose second has come.
pub(super) fn tick(world: &mut World, layer: Entity) {
    let now = world.resource::<Tick>().0;
    let due: Vec<(i64, f32, [f32; 3])> = world
        .query::<(&Dragon, &InLayer, &NetId, &Body, &Health)>()
        .iter(world)
        .filter(|(dragon, in_layer, ..)| in_layer.0 == layer && now >= dragon.born + FIRST && (now - dragon.born - FIRST) % EVERY == 0)
        .map(|(_, _, id, body, health)| (id.0 as i64, health.left() / health.max(), body.at))
        .collect();
    for (id, progress, at) in due {
        show(world, layer, id, TITLE, BossEventColor::Purple, progress, at, REACH);
    }
}

/// Shows the bar of the boss `id` at `at`, `progress` of it full, to the players of `layer` within `reach` of it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn show(world: &mut World, layer: Entity, id: i64, title: &str, color: BossEventColor, progress: f32, at: [f32; 3], reach: f32) {
    let near = |player: &Entity| world.get::<InLayer>(*player).is_some_and(|in_layer| in_layer.0 == layer) && world.get::<Position>(*player).is_some_and(|p| (0..3).map(|axis| (p.feet[axis] - at[axis]).powi(2)).sum::<f32>() <= reach * reach);
    let players: Vec<Entity> = in_world(world).into_iter().filter(near).collect();
    let bar = BossEvent { target_entity_id: id, r#type: BossEventType::ShowBar, title: title.into(), filtered_title: String::new(), progress, color, overlay: BossEventOverlay::Progress };
    for player in players {
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.push(&bar);
        }
    }
}
