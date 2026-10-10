//! `replay_check <seed> <world> <x0> <z0> <x1> <z1>`: with `SAPLING_ORACLE_ORDER` (and
//! `SAPLING_ORACLE_THREAD_SEED`) of the one-worker BDS run that saved `world`, every source in that
//! run's order with the cells it wrote last in the saved chunks and how many BDS has as another
//! block. The first source listed is where the port and that run part ways
//! (docs/research/worldgen-bds-order.md, "Replaying a run").
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::HashMap;

use oracle_support::{bds_cell, open, replay};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, x0, z0, x1, z1] = &args[..] else { panic!("usage: replay_check <seed> <world> <x0> <z0> <x1> <z1>") };
    let [x0, z0, x1, z1] = [x0, z0, x1, z1].map(|v| v.parse::<i32>().unwrap());
    let mut generator = OverworldGenerator::new(seed.parse().unwrap());
    let order = replay(&mut generator).expect("SAPLING_ORACLE_ORDER");
    let (primary, _) = open(world).expect("oracle world");
    let saved = |x: i32, z: i32| (x0..=x1).contains(&(x >> 4)) && (z0..=z1).contains(&(z >> 4));
    // cell -> (rank, block name) of the last source to write it
    let mut last = HashMap::new();
    for (rank, &(cx, cz)) in order.iter().enumerate() {
        if cx < x0 - 1 || cx > x1 + 1 || cz < z0 - 1 || cz > z1 + 1 {
            continue;
        }
        for (at, block, _) in generator.source_feature_writes(cx, cz) {
            if saved(at[0], at[2]) {
                last.insert(at, (rank, block));
            }
        }
    }
    let mut chunks = HashMap::new();
    let mut per_source: HashMap<usize, (u32, Vec<String>)> = HashMap::new();
    for (at, (rank, ours)) in last {
        let target = (at[0] >> 4, at[2] >> 4);
        let chunk = chunks.entry(target).or_insert_with(|| primary.load_column(ChunkKey::new(target.0, target.1, 0)).unwrap().expect("a saved chunk").chunk);
        let bds = bds_cell(&primary, chunk, at[0], at[1], at[2]).block;
        let entry = per_source.entry(rank).or_default();
        entry.0 += 1;
        if ours.name() != bds {
            entry.1.push(format!("{} {} {} {ours:?}>{bds}", at[0], at[1], at[2]));
        }
    }
    let mut ranks: Vec<_> = per_source.into_iter().collect();
    ranks.sort();
    for (rank, (cells, mut wrong)) in ranks {
        wrong.sort();
        let (cx, cz) = order[rank];
        println!("rank {rank} source {cx},{cz}: {cells} cells, {} differ {}", wrong.len(), wrong.iter().take(4).cloned().collect::<Vec<_>>().join("; "));
    }
}
