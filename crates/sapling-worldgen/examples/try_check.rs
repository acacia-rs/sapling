//! `try_check <seed> <world> <cx> <cz> <substring>`: the source chunk's rule tries whose trace line
//! holds the substring, each with what BDS saved in the cells the try wrote (and under it the same
//! per tree of a tree loop). The first try whose cells BDS has as other blocks is where the rule's
//! draws (or the world it saw) part ways.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::{BTreeMap, HashMap};

use acacia_world::Chunk;
use oracle_support::{bds_cell, open};
use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_worldgen::OverworldGenerator;

struct Saved<'a> {
    world: &'a LevelDbWorld,
    chunks: HashMap<(i32, i32), Option<Chunk>>,
}

impl Saved<'_> {
    /// `<n> writes: <our block>><BDS's block> <count>, ...` and the first cells that differ.
    fn summary(&mut self, writes: &[String]) -> String {
        let mut pairs: BTreeMap<String, u32> = BTreeMap::new();
        let mut differing = Vec::new();
        for write in writes {
            let mut parts = write.split(' ');
            let at: Vec<i32> = parts.by_ref().take(3).map(|v| v.parse().unwrap()).collect();
            let target = (at[0].div_euclid(16), at[2].div_euclid(16));
            let chunk = self.chunks.entry(target).or_insert_with(|| self.world.load_column(ChunkKey::new(target.0, target.1, 0)).unwrap().map(|c| c.chunk));
            let bds = chunk.as_ref().map_or("unsaved".to_string(), |chunk| bds_cell(self.world, chunk, at[0], at[1], at[2]).block);
            let ours = parts.next().unwrap_or("?");
            if !ours.to_lowercase().starts_with(&bds.replace('_', "")) && bds != "unsaved" && differing.len() < 4 {
                differing.push(format!("{at:?}"));
            }
            *pairs.entry(format!("{ours}>{bds}")).or_default() += 1;
        }
        let pairs: Vec<String> = pairs.iter().map(|(pair, n)| format!("{pair} {n}")).collect();
        format!("{} writes: {} {}", writes.len(), pairs.join(", "), differing.join(" "))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, cx, cz, want] = &args[..] else { panic!("usage: try_check <seed> <world> <cx> <cz> <substring>") };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let mut generator = OverworldGenerator::new(seed.parse().unwrap());
    // As the oracle test: a BDS run's decoration order and its one worker's thread generator.
    oracle_support::replay(&mut generator);
    let (primary, _) = open(world).expect("oracle world");
    let mut saved = Saved { world: &primary, chunks: HashMap::new() };
    let own = format!("({cx}, {cz}) ");
    // A try's write lines and its trees' lines come before its own line.
    let (mut writes, mut tree_writes, mut trees): (Vec<String>, Vec<String>, Vec<String>) = Default::default();
    for line in generator.source_feature_trace(cx, cz) {
        if let Some(write) = line.strip_prefix("    write ") {
            writes.push(write.to_string());
            tree_writes.push(write.to_string());
        } else if line.starts_with("  tree feature") {
            trees.push(format!("{line} | {}", saved.summary(&tree_writes)));
            tree_writes.clear();
        } else if !line.starts_with(' ') {
            if line.starts_with(&own) && line.contains(want.as_str()) {
                println!("{line} | {}", saved.summary(&writes));
                trees.iter().for_each(|tree| println!("{tree}"));
            }
            writes.clear();
            tree_writes.clear();
            trees.clear();
        }
    }
}
