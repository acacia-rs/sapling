//! `bds_blocks <world> <x0> <y0> <z0> <x1> <y1> <z1>`: how many of each block BDS saved in a box,
//! with the first position of each; finds the feature or structure behind a stretch of mismatches.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [world, rest @ ..] = &args[..] else { panic!("usage: bds_blocks <world> x0 y0 z0 x1 y1 z1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let (primary, _) = open(world).expect("oracle world");
    // BDS_STATES=1 counts each block with its states and liquid layer.
    let states = std::env::var_os("BDS_STATES").is_some();
    let mut counts: BTreeMap<String, (u32, (i32, i32, i32))> = BTreeMap::new();
    for x in n[0]..=n[3] {
        for z in n[2]..=n[5] {
            let chunk = primary.load_column(ChunkKey::new(x.div_euclid(16), z.div_euclid(16), 0)).unwrap().expect("chunk").chunk;
            for y in n[1]..=n[4] {
                let cell = bds_cell(&primary, &chunk, x, y, z);
                let key = if states { format!("{} [{}]{}", cell.block, cell.states, cell.liquid.map_or(String::new(), |l| format!(" +{l}"))) } else { cell.block };
                counts.entry(key).or_insert((0, (x, y, z))).0 += 1;
            }
        }
    }
    for (block, (count, first)) in counts {
        println!("{count:>7} {block} first {first:?}");
    }
}
