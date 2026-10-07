//! `column_pair <seed> <world> <x> <z>`: one column top down as runs of (ours before carvers,
//! ours generated, BDS) blocks.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, x, z] = &args[..] else { panic!("usage: column_pair <seed> <world> <x> <z>") };
    let (seed, x, z): (i64, i32, i32) = (seed.parse().unwrap(), x.parse().unwrap(), z.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    let chunk = primary.load_column(ChunkKey::new(x >> 4, z >> 4, 0)).unwrap().expect("chunk saved").chunk;
    let generator = OverworldGenerator::new(seed);
    let shape = generator.chunk_shape(x >> 4, z >> 4);
    let (surfaced, ours) = (generator.surfaced_blocks(x >> 4, z >> 4), generator.chunk_blocks(x >> 4, z >> 4));
    let (lx, lz) = ((x & 15) as usize, (z & 15) as usize);
    let mut top = 319;
    let label = |y| {
        let bds = bds_cell(&primary, &chunk, x, y, z);
        format!("{:?} {:?} {:?} {}[{}]", shape.kind(lx, y, lz), surfaced.block(lx, y, lz), ours.block(lx, y, lz), bds.block, bds.states)
    };
    for y in (-64..=319).rev() {
        if y == -64 || label(y - 1) != label(y) {
            println!("{y:>4}..={top:<4} {}", label(y));
            top = y - 1;
        }
    }
}
