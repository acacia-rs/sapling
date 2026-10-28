//! Fish out of water and fish that flee, as BDS's cod did in the `flop` drill
//! (research/captures/bds-flop.txt). On the ground a tick after it landed a fish hops 0.5 up and
//! up to 0.1 across, falling 0.1 a tick faster (`Physics::FISH`) and keeping 0.91 of its speed
//! across. In water one of the kinds whose pack has them avoid players
//! (`behavior.avoid_mob_type`, `max_dist` 6) swims off from a player within 6 to water further
//! from it, twice as fast as it swims while within 7 of the player and 1.5 times beyond.
// TODO: BDS's `flop` was heard for few of a cod's hops (a tropical fish's more); none is heard here.

use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use super::{GIVES_UP, REACHED, Swimmer, water_near};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Mode, Position, Tick, in_world};

const FISH: &[&str] = &["minecraft:cod", "minecraft:salmon", "minecraft:tropicalfish", "minecraft:pufferfish"];
const HOP_UP: f32 = 0.5;
const HOP_ACROSS: f32 = 0.1;
const FLEES_WITHIN: f32 = 6.0;
const SPRINTS_WITHIN: f32 = 7.0;
const SPRINT: f32 = 2.0;
const WALK: f32 = 1.5;
/// Where it flees to is looked for this far across and up or down, this many times.
const AWAY: [i32; 2] = [16, 4];
const AWAY_TRIES: usize = 10;

pub(in crate::creatures) fn is_fish(kind: &str) -> bool {
    FISH.contains(&kind)
}

/// Out of water: a fish a tick on the ground hops.
pub(super) fn flop(world: &mut World, entity: Entity, body: &Body) {
    let now = world.resource::<Tick>().0;
    let Some(mut swimmer) = world.get_mut::<Swimmer>(entity) else { return };
    if !body.on_ground {
        swimmer.landed = None;
        return;
    }
    match swimmer.landed {
        None => swimmer.landed = Some(now),
        Some(landed) if now > landed => {
            swimmer.landed = None;
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            let mut across = || (dice.fraction() * 2.0 - 1.0) * HOP_ACROSS;
            let hop = [body.motion[0] + across(), HOP_UP, body.motion[2] + across()];
            if let Some(mut body) = world.get_mut::<Body>(entity) {
                body.throw(hop);
            }
        }
        Some(_) => {}
    }
}

/// The nearest player within `within` of `at` in `layer` that is no spectator, and how far.
fn nearest_player(world: &World, layer: Entity, at: [f32; 3], within: f32) -> Option<([f32; 3], f32)> {
    in_world(world)
        .into_iter()
        .filter(|player| world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer) && world.get::<Mode>(*player).is_some_and(|mode| mode.0 != GameMode::Spectator))
        .filter_map(|player| world.get::<Position>(player).map(|position| position.feet))
        .map(|feet| (feet, (0..3).map(|axis| (feet[axis] - at[axis]).powi(2)).sum::<f32>().sqrt()))
        .filter(|(_, far)| *far <= within)
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// In water, before it swims: how many times its own pace a fish swims, fleeing a player near it
/// to a spot it is given; 1 where it flees nobody.
pub(super) fn flee(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body) -> f32 {
    if !is_fish(kind) {
        return 1.0;
    }
    let now = world.resource::<Tick>().0;
    let Some(swimmer) = world.get::<Swimmer>(entity) else { return 1.0 };
    let fled = swimmer.fleeing && swimmer.spot.is_some_and(|spot| now < swimmer.since + GIVES_UP && (0..3).map(|axis| (spot[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt() >= REACHED);
    if !fled {
        world.get_mut::<Swimmer>(entity).expect("read above").fleeing = false;
        let Some((threat, far)) = nearest_player(world, layer, body.at, FLEES_WITHIN) else { return 1.0 };
        let away = (0..AWAY_TRIES).filter_map(|_| water_near(world, layer, body.at, AWAY)).find(|spot| (0..3).map(|axis| (spot[axis] - threat[axis]).powi(2)).sum::<f32>().sqrt() > far);
        let Some(away) = away else { return 1.0 };
        let mut swimmer = world.get_mut::<Swimmer>(entity).expect("read above");
        (swimmer.spot, swimmer.since, swimmer.fleeing) = (Some(away), now, true);
    }
    if nearest_player(world, layer, body.at, SPRINTS_WITHIN).is_some() { SPRINT } else { WALK }
}
