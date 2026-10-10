//! `structure_replay_check <seed> <world> <x0> <z0> <x1> <z1>`: with `SAPLING_ORACLE_STRUCTURE_ORDER` of
//! the one-worker BDS run that saved `world`, the cells of the saved chunks where our structure stage
//! and that run disagree on a structure block (the oracle's `structure stage` tally), grouped by the
//! chunk and piece that wrote ours last, in the run's order: the first group is where the port and the
//! run part ways (docs/research/worldgen-bds-order.md, "Replaying a run"). Cells nothing of ours
//! wrote are listed under their own chunk's rank as `no write`. `SHOW=<n>` caps the groups (default 60),
//! `CELLS=<n>` the cells per group (default 6); `NEAR=x,y,z` prints every write within two blocks of a cell.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::{BTreeMap, HashMap};

use oracle_support::{PORTED, Stages, attribute, bds_cell, open, replay, stage_diff};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{MIN_Y, OverworldGenerator};

fn env(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: structure_replay_check <seed> <world> x0 z0 x1 z1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let mut generator = OverworldGenerator::new(seed.parse().unwrap());
    replay(&mut generator);
    let (primary, _) = open(world).expect("oracle world");
    let writes = generator.structure_replay_writes();
    assert!(!writes.is_empty(), "SAPLING_ORACLE_STRUCTURE_ORDER unset, or its run placed nothing");
    let mut last = HashMap::new();
    let mut ranks = HashMap::new();
    let near: Option<Vec<i32>> = std::env::var("NEAR").ok().map(|v| v.split(',').map(|n| n.parse().unwrap()).collect());
    for (i, write) in writes.iter().enumerate() {
        if near.as_ref().is_some_and(|n| (0..3).all(|k| (n[k] - write.at[k]).abs() <= 2)) {
            println!("near: rank {} chunk {:?} {:?} {}[{}] by {}", write.rank, write.chunk, write.at, write.block.name(), write.block.states(), write.label);
        }
        last.insert(write.at, i);
        ranks.entry(write.chunk).or_insert(write.rank);
    }
    let chunks: Vec<(i32, i32)> = (n[0]..=n[2]).flat_map(|x| (n[1]..=n[3]).map(move |z| (x, z))).collect();
    // (rank, first write of the label or MAX) -> (chunk, label, cells)
    let mut groups: BTreeMap<(usize, usize), ((i32, i32), String, Vec<String>)> = BTreeMap::new();
    let mut firsts: HashMap<(usize, String), usize> = HashMap::new();
    Stages::in_order(&generator, &chunks, |(cx, cz), Stages { loaded, structured, ours, areas, .. }| {
        let Some(saved) = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap() else { return };
        for x in cx * 16..cx * 16 + 16 {
            for z in cz * 16..cz * 16 + 16 {
                for y in MIN_Y..320 {
                    let (lx, lz) = ((x & 15) as usize, (z & 15) as usize);
                    let blocks = (loaded.block(lx, y, lz), structured.block(lx, y, lz), ours.block(lx, y, lz));
                    let bds = bds_cell(&primary, &saved.chunk, x, y, z);
                    let ported = areas.iter().any(|(kind, _, area)| PORTED.contains(kind) && area.contains(x, y, z));
                    let Some(diff) = stage_diff(blocks, &bds, attribute(&areas, (x, y, z)).is_some(), ported) else { continue };
                    let (rank, chunk, label, at) = match last.get(&[x, y, z]) {
                        Some(&i) => (writes[i].rank, writes[i].chunk, writes[i].label.to_string(), i),
                        None => (ranks.get(&(cx, cz)).copied().unwrap_or(usize::MAX), (cx, cz), "no write".to_string(), usize::MAX),
                    };
                    let first = *firsts.entry((rank, label.clone())).or_insert(at);
                    let cell = format!("{x} {y} {z} {diff:?} {}[{}]|{}[{}]", blocks.1.name(), blocks.1.states(), bds.block, bds.states);
                    groups.entry((rank, first)).or_insert_with(|| (chunk, label, Vec::new())).2.push(cell);
                }
            }
        }
    });
    let total: usize = groups.values().map(|g| g.2.len()).sum();
    println!("{total} cells differ in {} groups", groups.len());
    let mut per_chunk: BTreeMap<(usize, (i32, i32)), usize> = BTreeMap::new();
    for ((rank, _), (chunk, _, cells)) in &groups {
        *per_chunk.entry((*rank, *chunk)).or_default() += cells.len();
    }
    println!("by rank and chunk: {}", per_chunk.iter().map(|((rank, chunk), cells)| format!("{rank} {chunk:?} {cells}")).collect::<Vec<_>>().join("; "));
    for ((rank, _), (chunk, label, cells)) in groups.iter().take(env("SHOW", 60)) {
        println!("rank {rank} chunk {},{} | {label}: {} cells: {}", chunk.0, chunk.1, cells.len(), cells.iter().take(env("CELLS", 6)).cloned().collect::<Vec<_>>().join("; "));
    }
}
