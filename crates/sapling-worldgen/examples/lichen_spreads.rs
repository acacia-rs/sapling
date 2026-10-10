//! `lichen_spreads <seed> <world> <x0> <z0> <x1> <z1>`: every multiface spread of the source chunks
//! (trace lines `spread x y z from f to <to>:<cell>,<face> ...`) with, per candidate, whether BDS's
//! run and its twin hold a multiface block with that face there (`M`, `T`, `B` both, `-` neither).
//! Tells a rule difference (BDS always takes one of fewer candidates) from its thread-local pick.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::{BTreeMap, HashMap};

use acacia_world::Chunk;
use oracle_support::{bds_cell, open};
use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_worldgen::OverworldGenerator;

/// `multi_face_direction_bits` by FacingID (down, up, north, south, west, east).
const BITS: [u32; 6] = [1, 2, 16, 4, 8, 32];

struct Saved<'a> {
    world: &'a LevelDbWorld,
    chunks: HashMap<(i32, i32), Option<Chunk>>,
}

impl Saved<'_> {
    fn has_face(&mut self, [x, y, z]: [i32; 3], face: usize) -> bool {
        let target = (x.div_euclid(16), z.div_euclid(16));
        let chunk = self.chunks.entry(target).or_insert_with(|| self.world.load_column(ChunkKey::new(target.0, target.1, 0)).unwrap().map(|c| c.chunk));
        let Some(chunk) = chunk.as_ref() else { return false };
        let cell = bds_cell(self.world, chunk, x, y, z);
        let bits = cell.states.split("multi_face_direction_bits=").nth(1).and_then(|v| v.trim_end_matches(']').split(',').next()?.parse::<u32>().ok());
        bits.is_some_and(|bits| bits & BITS[face] != 0)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, x0, z0, x1, z1] = &args[..] else { panic!("usage: lichen_spreads <seed> <world> <x0> <z0> <x1> <z1>") };
    let n = |v: &String| v.parse::<i32>().unwrap();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, twin) = open(world).expect("oracle world");
    let mut main = Saved { world: &primary, chunks: HashMap::new() };
    let mut other = Saved { world: &twin, chunks: HashMap::new() };
    let mut tally: BTreeMap<String, u32> = BTreeMap::new();
    for cx in n(x0)..=n(x1) {
        for cz in n(z0)..=n(z1) {
            for line in generator.source_feature_trace(cx, cz) {
                let Some(rest) = line.strip_prefix("    spread ") else { continue };
                let (head, found) = rest.split_once(" to").unwrap();
                let marks: String = found
                    .split_whitespace()
                    .map(|candidate| {
                        let v: Vec<i32> = candidate.split_once(':').unwrap().1.split(',').map(|v| v.parse().unwrap()).collect();
                        let (m, t) = (main.has_face([v[0], v[1], v[2]], v[3] as usize), other.has_face([v[0], v[1], v[2]], v[3] as usize));
                        ['-', 'M', 'T', 'B'][usize::from(m) | usize::from(t) << 1]
                    })
                    .collect();
                if std::env::var_os("LIST").is_some() {
                    println!("{head} | {found} | {marks}");
                }
                *tally.entry(marks).or_default() += 1;
            }
        }
    }
    for (marks, count) in tally {
        println!("{count:6} {marks:?}");
    }
}
