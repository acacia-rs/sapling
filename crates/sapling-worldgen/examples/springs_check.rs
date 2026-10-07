//! `springs_check <seed> <world> <x0> <z0> <x1> <z1>`: liquid sources set into rock (springs),
//! ours against BDS's: cells whose terrain before features is rock and that end as a source.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{Block, OverworldGenerator};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: springs_check <seed> <world> x0 z0 x1 z1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    let mut springs: BTreeMap<(i32, i32, i32), (Option<&str>, Option<String>)> = BTreeMap::new();
    for cx in n[0]..=n[2] {
        for cz in n[1]..=n[3] {
            let a = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap().expect("chunk").chunk;
            let loaded = generator.loaded_blocks(cx, cz);
            let ours = generator.chunk_blocks(cx, cz);
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    for y in -64..256 {
                        if !loaded.block(x, y, z).is_solid() {
                            continue;
                        }
                        let bds = bds_cell(&primary, &a, wx, y, wz);
                        let mine = ours.block(x, y, z);
                        let ours_spring = matches!(mine, Block::Water | Block::Lava).then(|| mine.name());
                        let bds_spring = (bds.is_fluid() && bds.states == "liquid_depth=0").then(|| bds.block.clone());
                        if ours_spring.is_some() || bds_spring.is_some() {
                            springs.insert((wx, y, wz), (ours_spring, bds_spring));
                        }
                    }
                }
            }
        }
    }
    let both = springs.values().filter(|(a, b)| a.is_some() && a.map(str::to_string) == *b).count();
    println!("springs: {} ours, {} BDS, {both} both", springs.values().filter(|s| s.0.is_some()).count(), springs.values().filter(|s| s.1.is_some()).count());
    for (at, (ours, bds)) in &springs {
        if ours.map(str::to_string) != *bds {
            println!("  {at:?} ours {ours:?} bds {bds:?}");
        }
    }
}
