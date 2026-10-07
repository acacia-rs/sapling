//! `carver_misses <seed> <world> <x0> <z0> <x1> <z1>`: chunks ranked by carver disagreement
//! (cells BDS emptied that our carved shape keeps solid, and the reverse), with each chunk's y
//! range and a sample cell.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use oracle_support::{bds_cell, near_feature, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{CellKind, OverworldGenerator};

#[derive(Default)]
struct ChunkCount {
    missed: u32,
    extra: u32,
    y_range: (i32, i32),
    sample: Option<(i32, i32, i32)>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: carver_misses <seed> <world> x0 z0 x1 z1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, twin) = open(world).expect("oracle world");
    let mut counts = Vec::new();
    for cx in n[0]..=n[2] {
        for cz in n[1]..=n[3] {
            let key = ChunkKey::new(cx, cz, 0);
            let a = primary.load_column(key).unwrap().expect("chunk").chunk;
            let b = twin.load_column(key).unwrap().expect("twin").chunk;
            let uncarved = generator.chunk_shape(cx, cz);
            let mut carved = uncarved.clone();
            generator.carve(&mut carved, cx, cz);
            let mut count = ChunkCount { y_range: (i32::MAX, i32::MIN), ..Default::default() };
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    for y in -59..256 {
                        if uncarved.kind(x, y, z) != CellKind::Solid {
                            continue;
                        }
                        let bds = bds_cell(&primary, &a, wx, y, wz);
                        if bds.kind != bds_cell(&twin, &b, wx, y, wz).kind {
                            continue;
                        }
                        let (ours_empty, bds_empty) = (carved.kind(x, y, z) != CellKind::Solid, bds.kind != CellKind::Solid);
                        if ours_empty == bds_empty || (bds_empty && near_feature(&primary, &a, (wx, y, wz))) {
                            continue;
                        }
                        if bds_empty { count.missed += 1 } else { count.extra += 1 }
                        count.y_range = (count.y_range.0.min(y), count.y_range.1.max(y));
                        count.sample.get_or_insert((wx, y, wz));
                    }
                }
            }
            counts.push(((cx, cz), count));
        }
    }
    counts.sort_by_key(|(_, c)| std::cmp::Reverse(c.missed + c.extra));
    for ((cx, cz), c) in counts.iter().take(25) {
        println!("chunk {cx:>4},{cz:>4}: missed {:>5} extra {:>4} y {:?} sample {:?}", c.missed, c.extra, c.y_range, c.sample);
    }
}
