//! `try_check <seed> <world> <cx> <cz> <substring>`: the source chunk's rule tries whose trace line
//! holds the substring, each with what BDS saved in the cells the try wrote. The first try whose
//! cells BDS has as other blocks is where the rule's draws (or the world it saw) part ways.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::{BTreeMap, HashMap};

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, cx, cz, want] = &args[..] else { panic!("usage: try_check <seed> <world> <cx> <cz> <substring>") };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    let mut chunks = HashMap::new();
    let own = format!("({cx}, {cz}) ");
    // A try's write lines come before its own line.
    let mut writes: Vec<String> = Vec::new();
    for line in generator.source_feature_trace(cx, cz) {
        if let Some(write) = line.strip_prefix("    write ") {
            writes.push(write.to_string());
            continue;
        }
        if line.starts_with(' ') {
            continue;
        }
        if line.starts_with(&own) && line.contains(want.as_str()) {
            let mut saved: BTreeMap<String, u32> = BTreeMap::new();
            let mut differing = Vec::new();
            for write in &writes {
                let mut parts = write.split(' ');
                let at: Vec<i32> = parts.by_ref().take(3).map(|v| v.parse().unwrap()).collect();
                let target = (at[0].div_euclid(16), at[2].div_euclid(16));
                let chunk = chunks.entry(target).or_insert_with(|| primary.load_column(ChunkKey::new(target.0, target.1, 0)).unwrap().map(|c| c.chunk));
                let bds = chunk.as_ref().map_or("unsaved".to_string(), |chunk| bds_cell(&primary, chunk, at[0], at[1], at[2]).block);
                let ours = parts.next().unwrap_or("?");
                if ours.to_lowercase() != bds.replace('_', "") && differing.len() < 4 {
                    differing.push(format!("{at:?}"));
                }
                *saved.entry(format!("{ours}>{bds}")).or_default() += 1;
            }
            let saved: Vec<String> = saved.iter().map(|(pair, n)| format!("{pair} {n}")).collect();
            println!("{line} | {} writes: {} {}", writes.len(), saved.join(", "), differing.join(" "));
        }
        writes.clear();
    }
}
