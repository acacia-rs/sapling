//! Raids kept with the world, as BDS keeps them under `VILLAGE_<uuid>_RAID` (sapling-leveldb
//! raids.rs) with the keys its loader reads (0x142a52630): `State`, `Status`, `GroupNum`,
//! `NumGroups`, `GameTick`, `Ticks`, `Raiders` (their `UniqueID`s), `NumRaiders`,
//! `TotalMaxHealth`, `SpawnFails`. sapling keeps its own village beside them: `CenterX/Y/Z` and the
//! omen's `OmenLevel`. A kept raid's raiders rejoin as their columns load.
// TODO: BDS's `SpawnX/Y/Z` is its wave's spawn point, which sapling does not keep; the heroes are
// not kept either: a raid won after a load rewards the players in it then.

use acacia_proto::nbt::{self, LittleEndian, List, Nbt, Value};
use bevy_ecs::prelude::*;
use sapling_world::RaidRecord;

use super::raiders::Raider;
use super::{Raid, Stage};
use crate::components::Layer;
use crate::tags::UniqueId;

const LONG: u8 = 4;
/// BDS's `Raid::RaidState` and `RaidStatus`.
const PREPARING: i32 = 0;
const IN_PLAY: i32 = 3;
const FINISHED: i32 = 5;
const ONGOING: i32 = 0;
const VICTORY: i32 = 1;
const DEFEAT: i32 = 2;

/// The village's uuid, made of its middle.
fn village(middle: [f32; 3]) -> String {
    let [x, y, z] = middle.map(|c| c.floor() as i32);
    format!("{:08x}-{:04x}-4000-8000-{:012x}", x as u32, y as u16, z as u32)
}

fn encode(world: &World, raid: &Raid, now: u64) -> Vec<u8> {
    let (state, status, ticks) = match raid.stage {
        Stage::Gathering(until) => (PREPARING, ONGOING, until.saturating_sub(now)),
        Stage::Fighting => (IN_PLAY, ONGOING, 0),
        Stage::Won(since) => (FINISHED, VICTORY, now - since),
        Stage::Lost(since) => (FINISHED, DEFEAT, now - since),
    };
    let mut ids: Vec<i64> = raid.raiders.iter().filter_map(|raider| world.get::<UniqueId>(*raider)).map(|id| id.0).collect();
    ids.extend(&raid.unloaded);
    let [x, y, z] = raid.middle;
    let root = Value::Compound(vec![
        ("State".into(), Value::Int(state)),
        ("Status".into(), Value::Int(status)),
        ("GroupNum".into(), Value::Byte(raid.spawned as i8)),
        ("NumGroups".into(), Value::Byte(raid.waves as i8)),
        ("GameTick".into(), Value::Long(now as i64)),
        ("Ticks".into(), Value::Long(ticks as i64)),
        ("NumRaiders".into(), Value::Byte(ids.len() as i8)),
        ("Raiders".into(), Value::List(List { tag: LONG, items: ids.into_iter().map(Value::Long).collect() })),
        ("TotalMaxHealth".into(), Value::Float(raid.most)),
        ("SpawnFails".into(), Value::Byte(0)),
        ("CenterX".into(), Value::Float(x)),
        ("CenterY".into(), Value::Float(y)),
        ("CenterZ".into(), Value::Float(z)),
        ("OmenLevel".into(), Value::Byte(raid.level as i8)),
    ]);
    let mut out = bytes::BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: root });
    out.to_vec()
}

fn decode(layer: Entity, compound: &[u8], now: u64) -> Option<Raid> {
    let root = nbt::read::<LittleEndian>(&mut &compound[..]).ok()?.value;
    let int = |key: &str| match root.get(key) {
        Some(Value::Int(value)) => Some(i64::from(*value)),
        Some(Value::Byte(value)) => Some(i64::from(*value)),
        Some(Value::Long(value)) => Some(*value),
        _ => None,
    };
    let float = |key: &str| if let Some(Value::Float(value)) = root.get(key) { Some(*value) } else { None };
    let ticks = int("Ticks")?.max(0) as u64;
    let stage = match (int("State")? as i32, int("Status")? as i32) {
        (_, VICTORY) => Stage::Won(now.saturating_sub(ticks)),
        (_, DEFEAT) => Stage::Lost(now.saturating_sub(ticks)),
        (IN_PLAY, _) => Stage::Fighting,
        _ => Stage::Gathering(now + ticks),
    };
    let unloaded = match root.get("Raiders") {
        Some(Value::List(list)) => list.items.iter().filter_map(|id| if let Value::Long(id) = id { Some(*id) } else { None }).collect(),
        _ => Vec::new(),
    };
    let middle = [float("CenterX")?, float("CenterY")?, float("CenterZ")?];
    let level = int("OmenLevel").unwrap_or(1).clamp(1, i64::from(super::MOST)) as u8;
    let (spawned, waves, most) = (int("GroupNum")? as u8, int("NumGroups")? as u8, float("TotalMaxHealth").unwrap_or(1.0));
    Some(Raid { layer, middle, level, waves, spawned, stage, raiders: Vec::new(), unloaded, most, heroes: Vec::new() })
}

/// The overworld layers, where villages are.
fn overworlds(world: &mut World) -> Vec<Entity> {
    world.query::<(Entity, &Layer)>().iter(world).filter(|(_, layer)| layer.world.dimension() == 0).map(|(layer, _)| layer).collect()
}

/// The raids the world folders keep.
pub(super) fn load(world: &mut World, now: u64) -> Vec<Raid> {
    let mut raids = Vec::new();
    for layer in overworlds(world) {
        let records = world.get::<Layer>(layer).map(|kept| kept.world.level().raids.clone()).unwrap_or_default();
        raids.extend(records.iter().filter_map(|record| decode(layer, &record.compound, now)));
    }
    raids
}

/// Writes the raids into their world folders' level, for its next save.
pub(super) fn keep(world: &mut World, raids: &[Raid], now: u64) {
    for layer in overworlds(world) {
        let records: Vec<RaidRecord> = raids.iter().filter(|raid| raid.layer == layer).map(|raid| RaidRecord { village: village(raid.middle), compound: encode(world, raid, now) }).collect();
        if let Some(mut kept) = world.get_mut::<Layer>(layer) {
            kept.world.level_mut().raids = records;
        }
    }
}

/// A kept raid's raiders that have loaded since rejoin it.
pub(super) fn rejoin(world: &mut World, raid: &mut Raid) {
    if raid.unloaded.is_empty() {
        return;
    }
    let found: Vec<(Entity, i64)> = world.query::<(Entity, &UniqueId)>().iter(world).filter(|(_, id)| raid.unloaded.contains(&id.0)).map(|(raider, id)| (raider, id.0)).collect();
    for (raider, id) in found {
        raid.unloaded.retain(|unloaded| *unloaded != id);
        raid.raiders.push(raider);
        world.entity_mut(raider).insert(Raider { middle: raid.middle });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kept_raid_reads_back() {
        let mut world = World::new();
        let layer = world.spawn_empty().id();
        let raid = Raid { layer, middle: [3.5, 65.0, -3.5], level: 2, waves: 5, spawned: 2, stage: Stage::Gathering(150), raiders: Vec::new(), unloaded: vec![7, 9], most: 40.0, heroes: Vec::new() };
        let read = decode(layer, &encode(&world, &raid, 100), 1000).expect("readable");
        assert_eq!((read.middle, read.level, read.waves, read.spawned, read.stage, read.unloaded, read.most), (raid.middle, 2, 5, 2, Stage::Gathering(1050), vec![7, 9], 40.0));
        assert_eq!(village([3.5, 65.0, -3.5]), "00000003-0041-4000-8000-0000fffffffc");
    }
}
