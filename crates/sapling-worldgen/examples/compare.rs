//! `compare <seed> <oracle world dir name> <chunk x> <chunk z>`: per column of one chunk, the y
//! runs where sapling's cell kind differs from BDS's, as `y0..=y1 ours/bds`.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, cx, cz] = &args[..] else { panic!("usage: compare <seed> <world> <cx> <cz>") };
    let (seed, cx, cz): (i64, i32, i32) = (seed.parse().unwrap(), cx.parse().unwrap(), cz.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    let chunk = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap().expect("chunk saved").chunk;
    let shape = OverworldGenerator::new(seed).chunk_shape(cx, cz);
    for z in 0..16 {
        for x in 0..16 {
            let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
            let mut runs: Vec<(i32, i32, String)> = Vec::new();
            for y in -59..320 {
                let (ours, bds) = (shape.kind(x, y, z), bds_cell(&primary, &chunk, wx, y, wz));
                if ours == bds.kind {
                    continue;
                }
                let label = format!("{ours:?}/{}", bds.block);
                match runs.last_mut() {
                    Some(run) if run.1 == y - 1 && run.2 == label => run.1 = y,
                    _ => runs.push((y, y, label)),
                }
            }
            if !runs.is_empty() {
                let text: Vec<String> = runs.iter().map(|(a, b, l)| format!("{a}..={b} {l}")).collect();
                println!("{wx} {wz}: {}", text.join(", "));
            }
        }
    }
}
