//! `decoration_order <seed> <world> <cx0> <cz0> <cx1> <cz1>`: which of two neighbouring chunks BDS
//! decorated first, read off cells where both chunks' ore blobs claim the same stone (the first
//! one keeps it). Prints one line per chunk pair with votes: `pair ax az bx bz a_first b_first`.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::HashMap;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{Block, OverworldGenerator};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: decoration_order <seed> <world> <cx0> <cz0> <cx1> <cz1>") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, twin) = open(world).expect("oracle world");
    // cell -> (source, final block) of every source that claimed it over stone or deepslate.
    let mut claims: HashMap<[i32; 3], Vec<((i32, i32), Block)>> = HashMap::new();
    for cx in n[0]..=n[2] {
        for cz in n[1]..=n[3] {
            let mut own: HashMap<[i32; 3], Block> = HashMap::new();
            for (at, block, seen) in generator.source_feature_writes(cx, cz) {
                if matches!(seen, Block::Stone | Block::Deepslate) && !own.contains_key(&at) {
                    own.insert(at, block);
                }
            }
            for (at, block) in own {
                claims.entry(at).or_default().push(((cx, cz), block));
            }
        }
    }
    let mut votes: HashMap<((i32, i32), (i32, i32)), [u32; 2]> = HashMap::new();
    let mut chunks = HashMap::new();
    for (at, list) in claims.iter().filter(|(_, l)| l.len() == 2) {
        let [(a, ab), (b, bb)] = [list[0], list[1]];
        if ab == bb {
            continue;
        }
        let (a, ab, b, bb) = if a < b { (a, ab, b, bb) } else { (b, bb, a, ab) };
        let (cx, cz) = (at[0].div_euclid(16), at[2].div_euclid(16));
        let loaded = chunks.entry((cx, cz)).or_insert_with(|| {
            let key = ChunkKey::new(cx, cz, 0);
            Some((primary.load_column(key).unwrap()?.chunk, twin.load_column(key).unwrap()?.chunk))
        });
        let Some((p, t)) = loaded else { continue };
        let cell = bds_cell(&primary, p, at[0], at[1], at[2]);
        if cell != bds_cell(&twin, t, at[0], at[1], at[2]) {
            continue;
        }
        let v = votes.entry((a, b)).or_default();
        if cell.is(ab) {
            v[0] += 1;
        } else if cell.is(bb) {
            v[1] += 1;
        }
    }
    let mut pairs: Vec<_> = votes.into_iter().collect();
    pairs.sort();
    for ((a, b), [x, y]) in pairs {
        println!("pair {} {} {} {} {x} {y}", a.0, a.1, b.0, b.1);
    }
}
