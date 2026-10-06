//! `biome_column <seed> <oracle world dir name> <block x> <block z>`: sapling's and BDS's biome id
//! by y, as runs.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use oracle_support::open;
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{MIN_Y, OverworldGenerator};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, x, z] = &args[..] else { panic!("usage: biome_column <seed> <world> <x> <z>") };
    let (seed, x, z): (i64, i32, i32) = (seed.parse().unwrap(), x.parse().unwrap(), z.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    let chunk = primary.load_column(ChunkKey::new(x >> 4, z >> 4, 0)).unwrap().expect("chunk saved").chunk;
    let biomes = OverworldGenerator::new(seed).chunk_biomes(x >> 4, z >> 4);
    let cell = |y| (biomes.biome((x & 15) as usize, y, (z & 15) as usize), chunk.biome(x, y, z));
    let mut top = 319;
    for y in (MIN_Y..=319).rev() {
        if y == MIN_Y || cell(y - 1) != cell(y) {
            let (ours, bds) = cell(y);
            println!("{y:>4}..={top:<4} ours {ours:>3} bds {bds:?}");
            top = y - 1;
        }
    }
}
