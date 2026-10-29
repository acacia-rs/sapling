//! A player's standing with a village (BDS's `Village::mPlayerStanding`, docs/research/villagers.md
//! "Standing"): kept in [-30, 10] (`Village::modifyStanding`, 0x142a31380), a trade adds 1, a hit
//! on an adult villager takes 1 and on a baby 3, killing a villager takes 2 more and an iron golem
//! 5. Two village-wide timers (`Village::tick`, 0x142a23bc0) step every good standing down every
//! 120000 ticks and every bad one up every 12000. An iron golem takes the nearest player at -15 or
//! lower (`getClosestBadStandingPlayer`, 0x142a30c80), if it is the village's (golems.rs). Prices take no standing.
// TODO: standing is not kept with the world (BDS's `VILLAGE_<id>_PLAYERS`), and only melee hits
// count; BDS's other changes (0x142445c80 +5, 0x141ba8d80 +1, the founding reward) are not copied.

use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{Identity, InLayer, Position, Tick};

const LEAST: i32 = -30;
const MOST: i32 = 10;
const TRADED: i32 = 1;
const HURT: i32 = -1;
const HURT_BABY: i32 = -3;
const KILLED_VILLAGER: i32 = -2;
const KILLED_GOLEM: i32 = -5;
/// The standing an iron golem turns on a player at.
const OUTLAW: i32 = -15;
/// Ticks between a village's steps of good standings down, and of bad ones up.
const GOOD_FADES: u64 = 120_000;
const BAD_FADES: u64 = 12_000;
const VILLAGERS: [&str; 2] = ["minecraft:villager_v2", "minecraft:villager"];
const GOLEM: &str = "minecraft:iron_golem";

/// A player, by the uuid of its identity.
type Uuid = u128;

/// A village's standings by player, and the ticks its timers next step them.
#[derive(Clone, Debug, PartialEq)]
struct Village {
    players: HashMap<Uuid, i32>,
    good_due: u64,
    bad_due: u64,
}

impl Village {
    fn new(now: u64) -> Self {
        Self { players: HashMap::new(), good_due: now + GOOD_FADES, bad_due: now + BAD_FADES }
    }

    /// Steps its standings as its timers would have by `now`: as many steps as periods passed.
    fn settle(&mut self, now: u64) {
        if now >= self.good_due {
            let steps = ((now - self.good_due) / GOOD_FADES + 1) as i32;
            self.players.values_mut().filter(|value| **value > 0).for_each(|value| *value -= steps.min(*value));
            self.good_due += steps as u64 * GOOD_FADES;
        }
        if now >= self.bad_due {
            let steps = ((now - self.bad_due) / BAD_FADES + 1) as i32;
            self.players.values_mut().filter(|value| **value < 0).for_each(|value| *value += steps.min(-*value));
            self.bad_due += steps as u64 * BAD_FADES;
        }
    }

    fn change(&mut self, now: u64, player: Uuid, delta: i32) {
        self.settle(now);
        let value = self.players.entry(player).or_default();
        *value = (*value + delta).clamp(LEAST, MOST);
    }
}

/// Each village's standings, by world and village middle.
#[derive(Resource, Default)]
struct Standings(HashMap<(Entity, [i32; 3]), Village>);

fn village_of(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<[i32; 3]> {
    super::super::raids::village_centre(world, layer, at).map(|middle| middle.map(|c| c.floor() as i32))
}

/// The player's standing changes by `delta` with the village about `at`, if there is one.
fn change(world: &mut World, player: Entity, layer: Entity, at: [f32; 3], delta: i32) {
    let (Some(village), Some(uuid)) = (village_of(world, layer, at), world.get::<Identity>(player).map(|identity| identity.0.uuid.as_u128())) else { return };
    let now = world.resource::<Tick>().0;
    let mut standings = world.get_resource_or_insert_with(Standings::default);
    standings.0.entry((layer, village)).or_insert_with(|| Village::new(now)).change(now, uuid, delta);
}

/// The player traded with the villager `trader`.
pub(super) fn traded(world: &mut World, player: Entity, trader: Entity) {
    let (Some(layer), Some(at)) = (world.get::<InLayer>(trader).map(|layer| layer.0), world.get::<Body>(trader).map(|body| body.at)) else { return };
    change(world, player, layer, at, TRADED);
}

/// The player hit the mob `kind`, and `killed` it if so.
pub(in crate::creatures) fn hit(world: &mut World, player: Entity, mob: Entity, layer: Entity, kind: &str, killed: bool) {
    let Some(at) = world.get::<Body>(mob).map(|body| body.at) else { return };
    let delta = match kind {
        _ if VILLAGERS.contains(&kind) => {
            let baby = super::super::shown::is_baby(world, mob);
            if !baby {
                super::golems::defend(world, player, layer, at);
            }
            let hurt = if baby { HURT_BABY } else { HURT };
            hurt + if killed { KILLED_VILLAGER } else { 0 }
        }
        GOLEM if killed => KILLED_GOLEM,
        _ => return,
    };
    change(world, player, layer, at, delta);
}

/// For the iron golem `golem` at `body`, if its village's: the nearest player within `sight` in bad
/// standing with its village.
pub(in crate::creatures) fn outlaw(world: &mut World, golem: Entity, layer: Entity, kind: &str, body: &Body, sight: f32) -> Option<(Entity, [f32; 3])> {
    if kind != GOLEM || world.get::<super::golems::VillageGolem>(golem).is_none() {
        return None;
    }
    let key = (layer, village_of(world, layer, body.at)?);
    let now = world.resource::<Tick>().0;
    let mut village = world.get_resource_mut::<Standings>()?.0.get_mut(&key)?.clone();
    village.settle(now);
    let off = |at: [f32; 3]| (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt();
    let players: Vec<(Entity, [f32; 3])> = world
        .query::<(Entity, &InLayer, &Position, &Identity)>()
        .iter(world)
        .filter(|(_, on, at, identity)| on.0 == layer && off(at.feet) <= sight && village.players.get(&identity.0.uuid.as_u128()).is_some_and(|value| *value <= OUTLAW))
        .map(|(player, _, at, _)| (player, at.feet))
        .collect();
    world.resource_mut::<Standings>().0.insert(key, village);
    players.into_iter().filter(|(player, _)| crate::vitals::is_mortal(world, *player)).min_by(|a, b| off(a.1).total_cmp(&off(b.1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standing_is_clamped_and_drifts_back_on_the_villages_timers() {
        let (good, bad) = (1, 2);
        let mut village = Village::new(0);
        village.change(0, good, 40);
        village.change(0, bad, -40);
        assert_eq!((village.players[&good], village.players[&bad]), (MOST, LEAST));
        let at = |now| {
            let mut village = village.clone();
            village.settle(now);
            (village.players[&good], village.players[&bad])
        };
        assert_eq!([at(11_999), at(12_000), at(119_999), at(120_000), at(10_000_000)], [(10, -30), (10, -29), (10, -21), (9, -20), (0, 0)]);
    }
}
