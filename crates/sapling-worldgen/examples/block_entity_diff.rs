//! `block_entity_diff <seed> <world> [max chunks]`: every overworld chunk where BDS saved a
//! block entity the generator makes (spawners, loot, trial spawners, vaults, pots) against ours,
//! tallied per kind and table as `same`, `seed` (only the loot seed differs), `other`, `missing`
//! (BDS only) and `extra` (ours only); the first `other`, `missing` and `extra` are printed.
//! `<world>` is a folder under `$SAPLING_ORACLE_WORLDS` (e.g. 20261003-structures).

use std::collections::BTreeMap;
use std::path::PathBuf;

use acacia_proto::nbt::Value;
use sapling_leveldb::{LevelDbWorld, block_entities};
use sapling_worldgen::{BlockEntity, OverworldGenerator};

/// What we compare of an entity: its kind and table (or mob, configs, key), and its loot seed.
fn ours(entity: &BlockEntity) -> (String, Option<i32>) {
    match entity {
        BlockEntity::Spawner(mob) => (format!("spawner {mob}"), None),
        BlockEntity::Loot { table, seed } => (format!("loot {table}"), Some(*seed)),
        BlockEntity::Pot { table, seed, .. } => (format!("pot {}", table.unwrap_or("-")), table.map(|_| *seed)),
        BlockEntity::TrialSpawner { normal, .. } => (format!("trial {normal}"), None),
        BlockEntity::Vault { table, key } => (format!("vault {table} {key}"), None),
    }
}

fn text(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(s)) => Some(s.to_string()),
        _ => None,
    }
}

fn theirs(nbt: &Value) -> Option<(String, Option<i32>)> {
    let string = |key: &str| text(nbt.get(key));
    let seed = match nbt.get("LootTableSeed") {
        Some(Value::Int(seed)) => Some(*seed),
        _ => None,
    };
    let config = nbt.get("config");
    let table = text(config.and_then(|c| c.get("loot_table"))).unwrap_or_default();
    let key = text(config.and_then(|c| c.get("key_item")).and_then(|k| k.get("Name"))).unwrap_or_default();
    Some(match string("id")?.as_str() {
        "MobSpawner" => (format!("spawner {}", string("EntityIdentifier")?), None),
        "TrialSpawner" => (format!("trial {}", string("normal_config")?), None),
        "Vault" => (format!("vault {table} {key}"), None),
        "DecoratedPot" => (format!("pot {}", string("LootTable").unwrap_or("-".into())), seed),
        _ => (format!("loot {}", string("LootTable")?), seed),
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: block_entity_diff <seed> <world> [max chunks]") };
    let max: usize = rest.first().map_or(usize::MAX, |n| n.parse().unwrap());
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let root = std::env::var_os("SAPLING_ORACLE_WORLDS").map(PathBuf::from).expect("SAPLING_ORACLE_WORLDS");
    let primary = LevelDbWorld::open_in_memory(&root.join(world)).expect("the world");
    let mut tally: BTreeMap<(String, &str), u32> = BTreeMap::new();
    let (mut shown, mut chunks) = (0, 0);
    for key in primary.columns().unwrap().into_iter().filter(|key| key.dimension == 0) {
        let Some(column) = primary.load_column(key).unwrap() else { continue };
        let bds: BTreeMap<[i32; 3], (String, Option<i32>)> = block_entities(&column.block_entities).unwrap().iter().filter_map(|e| Some((e.pos, theirs(&e.nbt)?))).collect();
        if bds.is_empty() || chunks == max {
            continue;
        }
        chunks += 1;
        let shape = generator.chunk_blocks(key.x, key.z);
        let ours: BTreeMap<[i32; 3], (String, Option<i32>)> = shape.block_entities().iter().map(|([x, y, z], e)| ([key.x * 16 + x, *y, key.z * 16 + z], ours(e))).collect();
        for at in bds.keys().chain(ours.keys().filter(|at| !bds.contains_key(*at))) {
            let verdict = match (bds.get(at), ours.get(at)) {
                (Some(b), Some(o)) if b == o => "same",
                (Some(b), Some(o)) if b.0 == o.0 => "seed",
                (Some(_), Some(_)) => "other",
                (Some(_), None) => "missing",
                _ => "extra",
            };
            let kind = bds.get(at).or(ours.get(at)).unwrap().0.clone();
            *tally.entry((kind, verdict)).or_default() += 1;
            if !matches!(verdict, "same" | "seed") && shown < 40 {
                let block = shape.block((at[0] & 15) as usize, at[1], (at[2] & 15) as usize).name();
                println!("{at:?} {verdict}: BDS {:?} ours {:?} on our {block}", bds.get(at), ours.get(at));
                shown += 1;
            }
        }
    }
    println!("{chunks} chunks");
    for ((kind, verdict), count) in tally {
        println!("{count:>6} {verdict:<8} {kind}");
    }
}
