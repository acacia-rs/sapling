//! A player's standing with a village (BDS's `Village::mPlayerStanding`, docs/research/villagers.md
//! "Standing"): kept in [-30, 10] (`Village::modifyStanding`, 0x142a31380), a trade's level-up
//! adds 1 as it ends (trading.rs; a plain trade adds none), a hit
//! on an adult villager takes 1 and on a baby 3, by hand or by a shot, a killing hit 2 instead and
//! on an iron golem 5; a cure raises every standing the village keeps by 5, and so does a new
//! villager's first village (its founding reward). Two village-wide timers (`Village::tick`, 0x142a23bc0) step every good standing down every
//! 120000 ticks and every bad one up every 12000. An iron golem takes the nearest player at -15 or
//! lower (`getClosestBadStandingPlayer`, 0x142a30c80), if it is the village's (golems.rs). Prices take no standing.
//! Kept with the world as BDS's `VILLAGE_<id>_PLAYERS`, the village's id sapling's (raids/kept.rs).

use std::collections::HashMap;

use acacia_proto::nbt::{self, LittleEndian, List, Nbt, Value};
use bevy_ecs::prelude::*;
use sapling_world::{VillagePart, VillageRecord};

use crate::body::Body;
use crate::components::{InLayer, Layer, Position, Tick};
use crate::tags::Marks;

const COMPOUND: u8 = 10;

const LEAST: i32 = -30;
const MOST: i32 = 10;
const LEVELLED: i32 = 1;
const HURT: i32 = -1;
const HURT_BABY: i32 = -3;
const KILLED_VILLAGER: i32 = -2;
const KILLED_GOLEM: i32 = -5;
/// A zombie villager's cure: the conversion (0x142445c80, for zombie villager types 0x30b2c and
/// 0x30b74) finds the overworld village about it and calls `modifyStanding(+5)` over its whole
/// `mPlayerStanding` map (Village+0x218).
const CURED: i32 = 5;
/// A villager's first village gives every player it keeps a standing for its pack's
/// `first_founding_reward` (DwellerComponent 0x141ba80b0, once, `RewardPlayersOnFirstFounding`).
const FOUNDING: i32 = 5;
/// The standing an iron golem turns on a player at.
const OUTLAW: i32 = -15;
/// Ticks between a village's steps of good standings down, and of bad ones up.
const GOOD_FADES: u64 = 120_000;
const BAD_FADES: u64 = 12_000;
const VILLAGERS: [&str; 2] = ["minecraft:villager_v2", "minecraft:villager"];
const GOLEM: &str = "minecraft:iron_golem";

/// A player, as [`player_key`] names it.
type PlayerKey = i64;

/// A village's standings by player, and the ticks its timers next step them.
#[derive(Clone, Debug, PartialEq)]
struct Village {
    players: HashMap<PlayerKey, i32>,
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

    fn change(&mut self, now: u64, player: PlayerKey, delta: i32) {
        self.settle(now);
        let value = self.players.entry(player).or_default();
        *value = (*value + delta).clamp(LEAST, MOST);
    }

    /// Every player it keeps a standing for gains `delta`, whoever brought it.
    fn raise_all(&mut self, now: u64, delta: i32) {
        let players: Vec<PlayerKey> = self.players.keys().copied().collect();
        players.into_iter().for_each(|player| self.change(now, player, delta));
    }
}

/// Each village's standings, by world and village middle.
#[derive(Resource, Default)]
struct Standings(HashMap<(Entity, [i32; 3]), Village>);

/// The player's key in a village's standings: its `UniqueID`, as BDS keys them (bds-f14nvillage.txt).
fn player_key(world: &World, player: Entity) -> Option<PlayerKey> {
    world.get::<Marks>(player).map(|marks| marks.unique_id)
}

fn village_of(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<[i32; 3]> {
    super::super::raids::village_centre(world, layer, at).map(|middle| middle.map(|c| c.floor() as i32))
}

/// The standings, read from the world folders' `PLAYERS` records the first time.
fn standings(world: &mut World) -> Mut<'_, Standings> {
    if world.get_resource::<Standings>().is_none() {
        let now = world.resource::<Tick>().0;
        let layers: Vec<(Entity, Vec<VillageRecord>)> = world.query::<(Entity, &Layer)>().iter(world).map(|(layer, kept)| (layer, kept.world.level().villages.clone())).collect();
        let read = layers.into_iter().flat_map(|(layer, records)| {
            records.into_iter().filter(|record| record.part == VillagePart::Players).filter_map(move |record| {
                let middle = super::super::raids::village_middle(&record.village)?;
                Some(((layer, middle), Village { players: decode(&record.compound), ..Village::new(now) }))
            })
        });
        let standings = Standings(read.collect());
        world.insert_resource(standings);
    }
    world.resource_mut::<Standings>()
}

/// The player's standing changes by `delta` with the village about `at`, if there is one; the
/// village's standings are written into its world folder's level for its next save.
fn change(world: &mut World, player: Entity, layer: Entity, at: [f32; 3], delta: i32) {
    let (Some(middle), Some(key)) = (village_of(world, layer, at), player_key(world, player)) else { return };
    let now = world.resource::<Tick>().0;
    let mut standings = standings(world);
    let village = standings.0.entry((layer, middle)).or_insert_with(|| Village::new(now));
    village.change(now, key, delta);
    let compound = encode(&village.players);
    keep(world, layer, middle, compound);
}

/// A zombie villager was cured at `at`: the village there, if any, raises every standing it keeps.
pub(super) fn cured(world: &mut World, layer: Entity, at: [f32; 3]) {
    raise_all(world, layer, at, CURED);
}

/// A villager yet to give its founding reward is at `at`: the village there, if any, raises every
/// standing it keeps by [`FOUNDING`] (bds-y3found.txt: a hit's -1 became 4). False where there is none.
pub(super) fn founded(world: &mut World, layer: Entity, at: [f32; 3]) -> bool {
    raise_all(world, layer, at, FOUNDING)
}

/// The village about `at`, if any, raises every standing it keeps by `delta`; false where there is none.
fn raise_all(world: &mut World, layer: Entity, at: [f32; 3], delta: i32) -> bool {
    let Some(middle) = village_of(world, layer, at) else { return false };
    let now = world.resource::<Tick>().0;
    let mut standings = standings(world);
    let Some(village) = standings.0.get_mut(&(layer, middle)) else { return true };
    village.raise_all(now, delta);
    let compound = encode(&village.players);
    keep(world, layer, middle, compound);
    true
}

/// Writes the village's `PLAYERS` compound into its world folder's level for its next save.
fn keep(world: &mut World, layer: Entity, middle: [i32; 3], compound: Vec<u8>) {
    let Some(mut kept) = world.get_mut::<Layer>(layer) else { return };
    let uuid = super::super::raids::village_uuid(middle.map(|c| c as f32));
    let villages = &mut kept.world.level_mut().villages;
    villages.retain(|record| !(record.part == VillagePart::Players && record.village == uuid));
    villages.push(VillageRecord { village: uuid, part: VillagePart::Players, compound });
}

/// BDS's `PLAYERS` compound (0x142a2c140): `Players`, each player's `ID` and standing `S`.
fn encode(players: &HashMap<PlayerKey, i32>) -> Vec<u8> {
    let mut sorted: Vec<(&PlayerKey, &i32)> = players.iter().collect();
    sorted.sort();
    let items = sorted.into_iter().map(|(id, standing)| Value::Compound(vec![("ID".into(), Value::Long(*id)), ("S".into(), Value::Int(*standing))])).collect();
    let root = Value::Compound(vec![("Players".into(), Value::List(List { tag: COMPOUND, items }))]);
    let mut out = bytes::BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: root });
    out.to_vec()
}

fn decode(compound: &[u8]) -> HashMap<PlayerKey, i32> {
    let Ok(root) = nbt::read::<LittleEndian>(&mut &compound[..]).map(|read| read.value) else { return HashMap::new() };
    let Some(Value::List(players)) = root.get("Players") else { return HashMap::new() };
    let entry = |player: &Value| match (player.get("ID"), player.get("S")) {
        (Some(Value::Long(id)), Some(Value::Int(standing))) => Some((*id, *standing)),
        _ => None,
    };
    players.items.iter().filter_map(entry).collect()
}

/// The player's trade levelled up the villager `trader`, whose level-up ended.
pub(super) fn levelled(world: &mut World, player: Entity, trader: Entity) {
    let (Some(layer), Some(at)) = (world.get::<InLayer>(trader).map(|layer| layer.0), world.get::<Body>(trader).map(|body| body.at)) else { return };
    change(world, player, layer, at, LEVELLED);
}

/// What a player's hit on the mob `kind` does to the player's standing, if anything. A killing hit
/// counts as the kill alone: BDS's dweller takes the hurt only while it lives (0x148487bd0 returns
/// on health under 1), so a villager shot twice to death left -3 (bds-y3ranged.txt).
fn hit_delta(kind: &str, baby: bool, killed: bool) -> Option<i32> {
    match kind {
        _ if VILLAGERS.contains(&kind) && killed => Some(KILLED_VILLAGER),
        _ if VILLAGERS.contains(&kind) => Some(if baby { HURT_BABY } else { HURT }),
        GOLEM if killed => Some(KILLED_GOLEM),
        _ => None,
    }
}

/// The player hit the mob `kind` by hand, and `killed` it if so; a grown villager's village golem
/// turns on the player.
pub(in crate::creatures) fn hit(world: &mut World, player: Entity, mob: Entity, layer: Entity, kind: &str, killed: bool) {
    let Some(at) = world.get::<Body>(mob).map(|body| body.at) else { return };
    let baby = super::super::shown::is_baby(world, mob);
    if VILLAGERS.contains(&kind) && !baby {
        super::golems::defend(world, player, layer, at);
    }
    if let Some(delta) = hit_delta(kind, baby, killed) {
        change(world, player, layer, at, delta);
    }
}

/// The player's shot or throw hurt the mob `kind` at `at`, and `killed` it if so: the standing as a
/// hit's, but no golem turned on the archer (bds-y3ranged.txt: the held golem took no target).
pub(in crate::creatures) fn shot(world: &mut World, player: Entity, mob: Entity, layer: Entity, kind: &str, at: [f32; 3], killed: bool) {
    let baby = super::super::shown::is_baby(world, mob);
    if let Some(delta) = hit_delta(kind, baby, killed) {
        change(world, player, layer, at, delta);
    }
}

/// For the iron golem `golem` at `body`, if its village's: the nearest player within `sight` in bad
/// standing with its village.
pub(in crate::creatures) fn outlaw(world: &mut World, golem: Entity, layer: Entity, kind: &str, body: &Body, sight: f32) -> Option<(Entity, [f32; 3])> {
    if kind != GOLEM || world.get::<super::golems::VillageGolem>(golem).is_none() {
        return None;
    }
    let key = (layer, village_of(world, layer, body.at)?);
    let now = world.resource::<Tick>().0;
    let mut village = standings(world).0.get_mut(&key)?.clone();
    village.settle(now);
    let off = |at: [f32; 3]| (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt();
    let players: Vec<(Entity, [f32; 3])> = world
        .query::<(Entity, &InLayer, &Position, &Marks)>()
        .iter(world)
        .filter(|(_, on, at, marks)| on.0 == layer && off(at.feet) <= sight && village.players.get(&marks.unique_id).is_some_and(|value| *value <= OUTLAW))
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

    #[test]
    fn a_killing_hit_counts_as_the_kill_alone() {
        let villager = VILLAGERS[0];
        let shot_twice_to_death = hit_delta(villager, false, false).unwrap() + hit_delta(villager, false, true).unwrap();
        assert_eq!(shot_twice_to_death, -3, "bds-y3ranged.txt");
        assert_eq!([hit_delta(villager, true, false), hit_delta(villager, true, true), hit_delta(GOLEM, false, true), hit_delta(GOLEM, false, false)], [Some(HURT_BABY), Some(KILLED_VILLAGER), Some(KILLED_GOLEM), None]);
    }

    #[test]
    fn a_cure_raises_every_kept_standing_by_five() {
        let mut village = Village::new(0);
        village.change(0, 1, -12);
        village.change(0, 2, 8);
        village.raise_all(0, CURED);
        assert_eq!(village.players, HashMap::from([(1, -7), (2, MOST)]));
    }

    #[test]
    fn standings_read_back_from_bds_players_compound() {
        let players = HashMap::from([(-5, 3), (77, -20)]);
        assert_eq!(decode(&encode(&players)), players);
    }
}
