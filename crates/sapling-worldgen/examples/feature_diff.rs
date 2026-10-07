//! `feature_diff <seed> <world> <cx0> <cz0> <cx1> <cz1> <block>`: the cells where our generated block
//! (name without `minecraft:`) and BDS's disagree on being that block, column by column for a single
//! chunk, as a count for a range. With `LIST` set: every BDS cell of the block around the first chunk.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use oracle_support::{bds_cell, open};
use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_worldgen::{MIN_Y, OverworldGenerator};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, cx0, cz0, cx1, cz1, block] = &args[..] else { panic!("usage: feature_diff <seed> <world> <cx0> <cz0> <cx1> <cz1> <block>") };
    let n: Vec<i32> = [cx0, cz0, cx1, cz1].iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, twin) = open(world).expect("oracle world");
    if std::env::var_os("LIST").is_some() {
        list(&primary, n[0], n[1], block);
        return;
    }
    let single = n[0] == n[2] && n[1] == n[3];
    let mut counts = [0u64; 3];
    for cx in n[0]..=n[2] {
        for cz in n[1]..=n[3] {
            let before = counts;
            chunk_diff(&generator, (&primary, &twin), (cx, cz), block, single, &mut counts);
            if std::env::var_os("PER_CHUNK").is_some() {
                let biomes = generator.chunk_biomes(cx, cz);
                let mut ids: Vec<u32> = (MIN_Y..320).flat_map(|y| (0..256).map(move |i| (y, i))).map(|(y, i)| biomes.biome(i & 15, y, i >> 4)).collect();
                ids.sort_unstable();
                ids.dedup();
                println!("chunk {cx},{cz} biomes {ids:?}: same {}, only ours {}, only bds {}", counts[0] - before[0], counts[1] - before[1], counts[2] - before[2]);
            }
        }
    }
    println!("same {}, only ours {}, only bds {}", counts[0], counts[1], counts[2]);
}

fn chunk_diff(generator: &OverworldGenerator, (primary, twin): (&LevelDbWorld, &LevelDbWorld), (cx, cz): (i32, i32), block: &str, verbose: bool, counts: &mut [u64; 3]) {
    let ours = generator.chunk_blocks(cx, cz);
    let key = ChunkKey::new(cx, cz, 0);
    let chunk = primary.load_column(key).unwrap().expect("chunk").chunk;
    let other = twin.load_column(key).unwrap().expect("twin chunk").chunk;
    for z in 0..16 {
        for x in 0..16 {
            let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
            let mut line = String::new();
            for y in MIN_Y..320 {
                let mine = ours.block(x, y, z);
                let theirs = bds_cell(primary, &chunk, wx, y, wz);
                if theirs != bds_cell(twin, &other, wx, y, wz) {
                    continue;
                }
                match (mine.name() == block, theirs.block == block) {
                    (true, true) if theirs.is(mine) => counts[0] += 1,
                    (true, true) => line += &format!(" {y}:ours[{}]bds[{}]", mine.states(), theirs.states),
                    (true, false) => {
                        counts[1] += 1;
                        line += &format!(" {y}:ours[{}]bds={}", mine.states(), theirs.block);
                    }
                    (false, true) => {
                        counts[2] += 1;
                        line += &format!(" {y}:bds[{}]ours={}", theirs.states, mine.name());
                    }
                    _ => {}
                }
            }
            if verbose && !line.is_empty() {
                println!("{wx},{wz}:{line}");
            }
        }
    }
}

fn list(primary: &LevelDbWorld, cx: i32, cz: i32, block: &str) {
    for nx in cx - 1..=cx + 1 {
        for nz in cz - 1..=cz + 1 {
            let Ok(Some(column)) = primary.load_column(ChunkKey::new(nx, nz, 0)) else { continue };
            for y in MIN_Y..320 {
                for i in 0..256 {
                    let (x, z) = (nx * 16 + (i & 15), nz * 16 + (i >> 4));
                    let cell = bds_cell(primary, &column.chunk, x, y, z);
                    if cell.block == block {
                        println!("{x},{y},{z} {}", cell.states);
                    }
                }
            }
        }
    }
}
