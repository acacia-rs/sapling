//! `swap_check <seed> <world> <from> <to> <cx> <cz> <part,part...>`: checks a feature no oracle
//! world places. BDS generated `<world>` with a pack whose `features/<from>.json` carries `<to>`'s
//! body under `<from>`'s identifier (docs/research/worldgen-bds-trees.md, "Poplars"); here `<from>`
//! places `<to>` the same way. Over the 3x3 chunks around the chunk: the cells where either side
//! has a block whose name holds one of the parts, counted per `ours | BDS` pair, then the first
//! cells that differ. `<world>` needs a `-run2` twin beside it (a copy will do).
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{MIN_Y, OverworldGenerator};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, from, to, cx, cz, parts] = &args[..] else { panic!("usage: swap_check <seed> <world> <from> <to> <cx> <cz> <part,part...>") };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let parts: Vec<&str> = parts.split(',').collect();
    assert!(OverworldGenerator::swap_feature(from, to), "{from} and {to} must be jigsaw element features");
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, _) = open(world).expect("the BDS world and its -run2 twin");
    let (mut pairs, mut differing): (BTreeMap<String, u32>, Vec<String>) = Default::default();
    for (x, z) in (cx - 1..=cx + 1).flat_map(|x| (cz - 1..=cz + 1).map(move |z| (x, z))) {
        let Some(saved) = primary.load_column(ChunkKey::new(x, z, 0)).unwrap() else {
            println!("chunk {x},{z}: not saved");
            continue;
        };
        let shape = generator.chunk_blocks(x, z);
        for (lx, lz, y) in (0..16).flat_map(|lx| (0..16).flat_map(move |lz| (MIN_Y..320).map(move |y| (lx, lz, y)))) {
            let ours = shape.block(lx, y, lz);
            let at = [x * 16 + lx as i32, y, z * 16 + lz as i32];
            let bds = bds_cell(&primary, &saved.chunk, at[0], at[1], at[2]);
            if !parts.iter().any(|part| ours.name().contains(part) || bds.block.contains(part)) {
                continue;
            }
            let same = ours.name() == bds.block && ours.states().split(',').all(|state| bds.states.contains(state));
            if !same && differing.len() < 60 {
                differing.push(format!("{at:?} {}[{}] | {}[{}]", ours.name(), ours.states(), bds.block, bds.states));
            }
            *pairs.entry(format!("{} | {}{}", ours.name(), bds.block, if same { "" } else { "  <<<" })).or_default() += 1;
        }
    }
    pairs.iter().for_each(|(pair, n)| println!("{n:5} {pair}"));
    differing.iter().for_each(|line| println!("{line}"));
}
