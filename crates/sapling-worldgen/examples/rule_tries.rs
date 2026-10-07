//! `rule_tries <seed> <world> <cx> <cz> <rule name> <tries> <y0> <y1>`: the columns a feature rule
//! with uniform x, z over the chunk (drawn x then z) tries, as BDS and we left them: one glyph per
//! y from `y0` up (`.` air, `~` water, `L` log, `#` leaves, `r` roots, `m` mud, `v` vine, `o` other).
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;
use sapling_worldgen::random::{LargeFeatureSeed, Mt19937};

fn glyph(name: &str) -> char {
    match name {
        "air" => '.',
        "water" => '~',
        "mud" => 'm',
        "vine" => 'v',
        "mangrove_propagule" => 'p',
        "moss_carpet" => 'c',
        n if n.ends_with("_log") => 'L',
        n if n.ends_with("leaves") => '#',
        n if n.contains("roots") => 'r',
        _ => 'o',
    }
}

fn hashed_string(name: &str) -> u64 {
    name.bytes().fold(0xCBF2_9CE4_8422_2325u64, |hash, byte| hash.wrapping_mul(0x100_0000_01B3) ^ byte as u64)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, cx, cz, rule, tries, y0, y1] = &args[..] else { panic!("usage: rule_tries <seed> <world> <cx> <cz> <rule> <tries> <y0> <y1>") };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let (y0, y1): (i32, i32) = (y0.parse().unwrap(), y1.parse().unwrap());
    let seed: i64 = seed.parse().unwrap();
    let chunk_seed = LargeFeatureSeed::new(seed as u32).chunk(cx, cz);
    let hash = hashed_string(rule) as u32;
    let rule_seed = chunk_seed ^ hash.wrapping_add(0x9E37_79B9).wrapping_add(chunk_seed << 6).wrapping_add(chunk_seed >> 2);
    let mut mt = Mt19937::new(rule_seed);
    let generator = OverworldGenerator::new(seed);
    let (primary, _) = open(world).expect("oracle world");
    let shape = generator.chunk_blocks(cx, cz);
    let chunk = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap().expect("chunk").chunk;
    for i in 0..tries.parse::<u32>().unwrap() {
        let lx = mt.next_int(16);
        let lz = mt.next_int(16);
        let (x, z) = (cx * 16 + lx, cz * 16 + lz);
        let ours: String = (y0..=y1).map(|y| glyph(shape.block(lx as usize, y, lz as usize).name())).collect();
        let bds: String = (y0..=y1).map(|y| glyph(&bds_cell(&primary, &chunk, x, y, z).block)).collect();
        println!("{i:>2} {x},{z} ours {ours} bds {bds}{}", if ours == bds { "" } else { "  *" });
    }
}
