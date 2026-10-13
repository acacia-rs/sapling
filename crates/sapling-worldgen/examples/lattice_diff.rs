//! `lattice_diff <seed> <world> <chunk x0> <chunk z0> <chunk x1> <chunk z1>`: BDS's biome at each
//! 8-block lattice point, inferred from the blocks that pick it (the jittered pick is shared),
//! against sapling's; prints the lattice points that disagree.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::open;
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{MIN_Y, OverworldGenerator};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, x0, z0, x1, z1] = &args[..] else { panic!("usage: lattice_diff <seed> <world> <x0> <z0> <x1> <z1>") };
    let seed: i64 = seed.parse().unwrap();
    let [x0, z0, x1, z1]: [i32; 4] = [x0, z0, x1, z1].map(|v| v.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    let generator = OverworldGenerator::new(seed);
    // lattice point -> (ours, BDS biome -> votes)
    let mut points: BTreeMap<[i32; 3], (u32, BTreeMap<u32, u32>)> = BTreeMap::new();
    for cx in x0..=x1 {
        for cz in z0..=z1 {
            let chunk = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap().expect("chunk saved").chunk;
            let biomes = generator.density().climate.chunk_biomes_traced(seed, cx, cz);
            for y in MIN_Y..64 {
                for z in 0..16 {
                    for x in 0..16 {
                        let Some(bds) = chunk.biome(cx * 16 + x as i32, y, cz * 16 + z as i32) else { continue };
                        let Some(point) = biomes.lattice_point(x, y, z) else { continue };
                        let entry = points.entry(point).or_insert((biomes.biome(x, y, z), BTreeMap::new()));
                        *entry.1.entry(bds).or_default() += 1;
                    }
                }
            }
        }
    }
    let mut differing = 0;
    for (point, (ours, votes)) in &points {
        if votes.len() != 1 || !votes.contains_key(ours) {
            differing += 1;
            if differing <= 40 {
                println!("lattice {point:?}: ours {ours} bds {votes:?}");
            }
        }
    }
    println!("{differing} of {} lattice points differ", points.len());
}
