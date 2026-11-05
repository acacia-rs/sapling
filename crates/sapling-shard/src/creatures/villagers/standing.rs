//! A player's standing with a village (BDS's `Village::mPlayerStanding`, docs/research/villagers.md
//! "Standing"): kept in [-30, 10] (`Village::modifyStanding`, 0x142a31380), a trade's level-up
//! adds 1 as it ends (trading.rs; a plain trade adds none), a hit
//! on an adult villager takes 1 and on a baby 3, killing a villager takes 2 more and an iron golem
//! 5; a cure raises every standing the village keeps by 5. Two village-wide timers (`Village::tick`, 0x142a23bc0) step every good standing down every
//! 120000 ticks and every bad one up every 12000. An iron golem takes the nearest player at -15 or
//! lower (`getClosestBadStandingPlayer`, 0x142a30c80), if it is the village's (golems.rs). Prices take no standing.
//! Kept with the world as BDS's `VILLAGE_<id>_PLAYERS`, the village's id sapling's (raids/kept.rs).
// TODO: only melee hits count; BDS's founding reward is not copied.

use std::collections::HashMap;

use acacia_proto::nbt::{self, LittleEndian, List, Nbt, Value};
use bevy_ecs::prelude::*;
use sapling_world::{VillagePart, VillageRecord};

use crate::body::Body;
use crate::components::{Identity, InLayer, Layer, Position, Tick};

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

    /// Every player it keeps a standing for gains [`CURED`], whoever cured.
    fn cured(&mut self, now: u64) {
        let players: Vec<PlayerKey> = self.players.keys().copied().collect();
        players.into_iter().for_each(|player| self.change(now, player, CURED));
    }
}

/// Each village's standings, by world and village middle.
#[derive(Resource, Default)]
struct Standings(HashMap<(Entity, [i32; 3]), Village>);

/// The player's key in a village's standings: BDS keys them by `ActorUniqueID`, which sapling's
/// players do not keep, so the low half of the identity's uuid stands in for it.
fn player_key(world: &World, player: Entity) -> Option<PlayerKey> {
    world.get::<Identity>(player).map(|identity| identity.0.uuid.as_u128() as i64)
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
    let Some(middle) = village_of(world, layer, at) else { return };
    let now = world.resource::<Tick>().0;
    let mut standings = standings(world);
    let Some(village) = standings.0.get_mut(&(layer, middle)) else { return };
    village.cured(now);
    let compound = encode(&village.players);
    keep(world, layer, middle, compound);
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
    let mut village = standings(world).0.get_mut(&key)?.clone();
    village.settle(now);
    let off = |at: [f32; 3]| (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt();
    let players: Vec<(Entity, [f32; 3])> = world
        .query::<(Entity, &InLayer, &Position, &Identity)>()
        .iter(world)
        .filter(|(_, on, at, identity)| on.0 == layer && off(at.feet) <= sight && village.players.get(&(identity.0.uuid.as_u128() as i64)).is_some_and(|value| *value <= OUTLAW))
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
    fn a_cure_raises_every_kept_standing_by_five() {
        let mut village = Village::new(0);
        village.change(0, 1, -12);
        village.change(0, 2, 8);
        village.cured(0);
        assert_eq!(village.players, HashMap::from([(1, -7), (2, MOST)]));
    }

    #[test]
    fn standings_read_back_from_bds_players_compound() {
        let players = HashMap::from([(-5, 3), (77, -20)]);
        assert_eq!(decode(&encode(&players)), players);
    }
}
