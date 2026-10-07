//! `tree_draws <seed> <world> <cx> <cz> <rule name>`: for a chunk decorated by a `LegacyTreeFeature`
//! rule, BDS's trunks in draw order: where each one's `z, x` (`nextInt(14) + 1` each) sits in the
//! rule's placing generator, so the gaps show how many draws every tree (and every failed attempt
//! between them) spent.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::random::{LargeFeatureSeed, Mt19937};

fn hashed_string(name: &str) -> u64 {
    name.bytes().fold(0xCBF2_9CE4_8422_2325u64, |hash, byte| hash.wrapping_mul(0x100_0000_01B3) ^ byte as u64)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, cx, cz, rule] = &args[..] else { panic!("usage: tree_draws <seed> <world> <cx> <cz> <rule>") };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let seed: i64 = seed.parse().unwrap();
    let chunk_seed = LargeFeatureSeed::new(seed as u32).chunk(cx, cz);
    let hash = hashed_string(rule) as u32;
    let rule_seed = chunk_seed ^ hash.wrapping_add(0x9E37_79B9).wrapping_add(chunk_seed << 6).wrapping_add(chunk_seed >> 2);
    let mut mt = Mt19937::new(rule_seed);
    let draws: Vec<u32> = (0..4000).map(|_| mt.next_u32()).collect();
    let (primary, _) = open(world).expect("oracle world");
    let chunk = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap().expect("chunk").chunk;
    let mut trunks = Vec::new();
    for lz in 1..15 {
        for lx in 1..15 {
            let (x, z) = (cx * 16 + lx, cz * 16 + lz);
            for y in -63..320 {
                let cell = bds_cell(&primary, &chunk, x, y, z);
                let below = bds_cell(&primary, &chunk, x, y - 1, z);
                if cell.block.ends_with("_log") && cell.states.contains("=y") && !below.block.ends_with("_log") {
                    let mut top = y;
                    while bds_cell(&primary, &chunk, x, top + 1, z).block == cell.block {
                        top += 1;
                    }
                    trunks.push((lz as u32 - 1, lx as u32 - 1, [x, y, z], cell.block.clone(), top - y + 1));
                }
            }
        }
    }
    let mut from = 2;
    let mut left: Vec<_> = trunks.clone();
    while !left.is_empty() && from + 1 < draws.len() {
        // With HEIGHT=<base>:<span> a trunk also needs `base + draw % span` at the fourth draw (gate, then height).
        let height_ok = |h: i32| match std::env::var("HEIGHT") {
            Ok(spec) => {
                let (base, span) = spec.split_once(':').unwrap();
                h == base.parse::<i32>().unwrap() + (draws[from + 3] % span.parse::<u32>().unwrap()) as i32
            }
            Err(_) => true,
        };
        if let Some(k) = left.iter().position(|&(z, x, _, _, h)| draws[from] % 14 == z && draws[from + 1] % 14 == x && height_ok(h)) {
            let (_, _, at, block, height) = left.remove(k);
            println!("draw {from}: {at:?} {block} h{height} (next %80 {} %3 {})", draws[from + 2] % 80, draws[from + 3] % 3);
        }
        from += 1;
    }
    for (_, _, at, block, height) in left {
        println!("not found: {at:?} {block} h{height}");
    }
}
