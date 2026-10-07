//! `find_block <world> <x0> <z0> <x1> <z1> <block> <min y>`: per chunk, how many cells of a block
//! (name without `minecraft:`, liquids included) BDS saved at or above `min y`, and the first one.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [world, x0, z0, x1, z1, block, min_y] = &args[..] else { panic!("usage: find_block <world> x0 z0 x1 z1 <block> <min y>") };
    let n: Vec<i32> = [x0, z0, x1, z1, min_y].iter().map(|v| v.parse().unwrap()).collect();
    let (primary, _) = open(world).expect("oracle world");
    for cx in n[0]..=n[2] {
        for cz in n[1]..=n[3] {
            let chunk = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap().expect("chunk").chunk;
            let (mut count, mut first) = (0, None);
            for y in n[4]..320 {
                for i in 0..256 {
                    let (x, z) = (cx * 16 + (i & 15), cz * 16 + (i >> 4));
                    if bds_cell(&primary, &chunk, x, y, z).block.trim_start_matches("flowing_") == block.as_str() {
                        count += 1;
                        first.get_or_insert((x, y, z));
                    }
                }
            }
            if count > 0 {
                println!("chunk {cx},{cz}: {count} first {first:?}");
            }
        }
    }
}
