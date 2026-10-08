//! `structure_diff <seed> <world> <x0> <y0> <z0> <x1> <y1> <z1>`: cells in a box where our
//! structure stage or BDS put a structure block and the two disagree, as `x y z ours | BDS`,
//! with the counts of each pair. `PALETTE=a,b` sets the name parts that make a BDS block a
//! structure block (default: mineshaft blocks), `ANY` lets terrain blocks count too, and
//! `EXCLUDE=a,b` drops cells where either block's name has one of those parts.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{TERRAIN_BLOCKS, bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: structure_diff <seed> <world> x0 y0 z0 x1 y1 z1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    // Blocks of the ported structures; other non-terrain blocks are later stages'.
    let palette = std::env::var("PALETTE").unwrap_or("planks,fence,rail,torch,web,chain,_log,mob_spawner,chest".into());
    let palette: Vec<&str> = palette.split(',').collect();
    let structural = |name: &str| (!TERRAIN_BLOCKS.contains(&name) || std::env::var_os("ANY").is_some()) && palette.iter().any(|p| name.contains(p));
    let exclude = std::env::var("EXCLUDE").unwrap_or_default();
    let excluded = |name: &str| exclude.split(',').any(|p| !p.is_empty() && name.contains(p));
    let mut pairs: BTreeMap<(String, String), u32> = BTreeMap::new();
    let mut shown = 0;
    for cx in n[0].div_euclid(16)..=n[3].div_euclid(16) {
        for cz in n[2].div_euclid(16)..=n[5].div_euclid(16) {
            let ours = generator.loaded_blocks(cx, cz);
            let mut before = generator.surfaced_blocks(cx, cz);
            generator.carve(&mut before, cx, cz);
            let Some(column) = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap() else { continue };
            for x in (cx * 16).max(n[0])..=(cx * 16 + 15).min(n[3]) {
                for z in (cz * 16).max(n[2])..=(cz * 16 + 15).min(n[5]) {
                    for y in n[1]..=n[4] {
                        let block = ours.block((x & 15) as usize, y, (z & 15) as usize);
                        let bds = bds_cell(&primary, &column.chunk, x, y, z);
                        let changed = before.block((x & 15) as usize, y, (z & 15) as usize) != block;
                        if (changed || structural(&bds.block)) && !bds.is(block) && !excluded(block.name()) && !excluded(&bds.block) {
                            let ours = format!("{}[{}]", block.name(), block.states());
                            let theirs = format!("{}[{}]", bds.block, bds.states);
                            if shown < 200 {
                                println!("{x} {y} {z} {ours} | {theirs}");
                                shown += 1;
                            }
                            *pairs.entry((ours, theirs)).or_default() += 1;
                        }
                    }
                }
            }
        }
    }
    let mut sorted: Vec<_> = pairs.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    for ((ours, theirs), count) in sorted.iter().take(40) {
        println!("{count:>6} {ours} | {theirs}");
    }
}
