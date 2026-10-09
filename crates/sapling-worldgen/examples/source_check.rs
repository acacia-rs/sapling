//! `source_check <seed> <world> <cx> <cz> [LIST]`: one source chunk's feature writes against BDS's
//! saved cells, per target chunk and written block: how many BDS has (`same`), has as another
//! block, or lacks. Tells whether BDS ran a source (a ring chunk, say) and saw the same world; with
//! `LIST`, each write BDS does not have.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::{BTreeMap, HashMap};

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, cx, cz, rest @ ..] = &args[..] else { panic!("usage: source_check <seed> <world> <cx> <cz> [LIST]") };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    // The last write of each cell.
    let mut last = HashMap::new();
    for (at, block, _) in generator.source_feature_writes(cx, cz) {
        last.insert(at, block);
    }
    let mut chunks = HashMap::new();
    let mut counts: BTreeMap<((i32, i32), &str), [u32; 3]> = BTreeMap::new();
    for (at, block) in last {
        let target = (at[0].div_euclid(16), at[2].div_euclid(16));
        let saved = chunks.entry(target).or_insert_with(|| primary.load_column(ChunkKey::new(target.0, target.1, 0)).unwrap().map(|c| c.chunk));
        let Some(chunk) = saved else { continue };
        let cell = bds_cell(&primary, chunk, at[0], at[1], at[2]);
        let kind = if cell.is(block) { 0 } else if cell.block == block.name() { 1 } else { 2 };
        counts.entry((target, block.name())).or_default()[kind] += 1;
        if kind == 2 && !rest.is_empty() {
            println!("{} {} {} ours {} bds {}", at[0], at[1], at[2], block.name(), cell.block);
        }
    }
    for ((target, name), [same, states, other]) in counts {
        println!("into {target:?} {name:>24}: same {same:>5}, other states {states:>4}, not in BDS {other:>5}");
    }
}
