//! A bee's anger and sting (bees.rs): hit, it goes for its hitter with every bee within 20, as one
//! within 16 of a broken hive goes for the nearest player within 10; its sting poisons on normal
//! (10 s) and hard (18 s), and it dies 10 to 60 s after.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Difficulty};

use super::super::hunting;
use super::{Bee, KIND, distance, middle};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, NetId, Position, Settings, Tick};
use crate::entities::Kind;

/// Ticks from its sting to its death, the least and the most.
const PERISHES: [u32; 2] = [200, 1200];
/// Its anger reaches bees this near; a broken hive angers those this near it at players this near them.
const ANGER_SPREADS: f32 = 20.0;
const HIVE_SENSES: f32 = 16.0;
const PICKS_PLAYER: f32 = 10.0;
/// Poison its sting gives on normal and hard.
const POISONS: [u32; 2] = [200, 360];

/// A hunter swung at `prey` (hit or not, as `bitten`): a bee's sting poisons on normal and hard,
/// and the bee dies [`PERISHES`] after, hunting no more.
pub(in crate::creatures) fn stung(world: &mut World, bee: Entity, prey: Entity, bitten: bool) {
    if world.get::<Bee>(bee).is_none() {
        return;
    }
    let poison = match world.resource::<Settings>().0.difficulty {
        Difficulty::Normal => Some(POISONS[0]),
        Difficulty::Hard => Some(POISONS[1]),
        _ => None,
    };
    if let Some(ticks) = poison.filter(|_| bitten) {
        crate::vitals::afflict(world, prey, "poison", ticks);
    }
    let after = PERISHES[0] + world.get_resource_or_insert_with(Dice::default).roll(PERISHES[1] - PERISHES[0] + 1);
    let now = world.resource::<Tick>().0;
    if let Some(mut bee) = world.get_mut::<Bee>(bee) {
        bee.perishes = Some(now + u64::from(after));
    }
}

/// Whether the bee stung and hunts no more.
pub(in crate::creatures) fn is_spent(world: &World, bee: Entity) -> bool {
    world.get::<Bee>(bee).is_some_and(|bee| bee.perishes.is_some())
}

/// Bees within `reach` of `at` in `layer`, but `but`, that have not stung.
fn bees_near(world: &mut World, layer: Entity, at: [f32; 3], reach: f32, but: Option<Entity>) -> Vec<(Entity, u64, Body)> {
    world
        .query::<(Entity, &InLayer, &NetId, &Kind, &Body, &Bee)>()
        .iter(world)
        .filter(|(entity, in_layer, _, kind, body, bee)| in_layer.0 == layer && kind.0 == KIND && Some(*entity) != but && bee.perishes.is_none() && distance(body.at, at) <= reach)
        .map(|(entity, _, id, _, body, _)| (entity, id.0, *body))
        .collect()
}

/// A hunter took `player` for its prey: if a bee, so does every bee within 20 of it (`broadcastAnger`).
pub(in crate::creatures) fn spread_anger(world: &mut World, bee: Entity, layer: Entity, at: [f32; 3], player: Entity) {
    if world.get::<Bee>(bee).is_none() {
        return;
    }
    for (other, id, body) in bees_near(world, layer, at, ANGER_SPREADS, Some(bee)) {
        hunting::provoke(world, other, layer, id, KIND, &body, player);
    }
}

/// The hive at `pos` was broken: each bee within 16 goes for the nearest player within 10 of it.
pub(super) fn disturbed(world: &mut World, layer: Entity, pos: BlockPos) {
    for (bee, id, body) in bees_near(world, layer, middle(pos), HIVE_SENSES, None) {
        let players = crate::components::in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).is_some_and(|in_layer| in_layer.0 == layer) && crate::vitals::is_mortal(world, *player));
        let nearest = players.filter_map(|player| Some((player, distance(world.get::<Position>(player)?.feet, body.at)))).filter(|(_, far)| *far <= PICKS_PLAYER).min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((player, _)) = nearest {
            hunting::provoke(world, bee, layer, id, KIND, &body, player);
        }
    }
}
