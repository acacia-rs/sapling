//! `structure_cells <seed> <world> <cx0> <cz0> <cx1> <cz1>`: every stable cell of the chunks where our
//! structure stage and BDS disagree on a structure block (the oracle's `structure stage` tally),
//! with what wrote ours (structure/trace.rs: piece, template cell and its block before the rules),
//! then the counts by writer and block pair. `SHOW=<n>` caps the cell lines (default 300);
//! `ONLY=<substring>` keeps the cells whose line has it.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::{BTreeMap, HashMap};

use oracle_support::{Stages, attribute, bds_cell, open, stage_diff};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{MIN_Y, OverworldGenerator};

/// A template label `<name> <rotation> t(<x>, <y>, <z>) <raw block>` as (name, raw block); others whole.
fn writer(label: &str) -> (String, String) {
    let Some((head, rest)) = label.split_once(" t(") else { return (label.to_string(), String::new()) };
    let name = head.rsplit_once(' ').map_or(head, |(name, _)| name);
    (name.to_string(), rest.split_once(") ").map_or("", |(_, raw)| raw).to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: structure_cells <seed> <world> cx0 cz0 cx1 cz1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, twin) = open(world).expect("oracle world");
    let show: usize = std::env::var("SHOW").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    let only = std::env::var("ONLY").ok();
    let chunks: Vec<(i32, i32)> = (n[0]..=n[2]).flat_map(|x| (n[1]..=n[3]).map(move |z| (x, z))).collect();
    let mut groups: BTreeMap<(String, String, String, String), u32> = BTreeMap::new();
    let mut kinds: BTreeMap<String, u32> = BTreeMap::new();
    let mut per_chunk: BTreeMap<(i32, i32), u32> = BTreeMap::new();
    let (mut shown, mut unstable) = (0, 0);
    Stages::in_order(&generator, &chunks, |(cx, cz), Stages { loaded, structured, ours, areas, .. }| {
        let key = ChunkKey::new(cx, cz, 0);
        let Some(a) = primary.load_column(key).unwrap() else { return };
        let b = twin.load_column(key).unwrap().expect("twin chunk saved");
        let mut labels: HashMap<[i32; 3], Vec<String>> = HashMap::new();
        for ((at, _), label) in generator.structure_trace(cx, cz) {
            let list = labels.entry(at).or_default();
            if list.last() != Some(&label) {
                list.push(label);
            }
        }
        for x in cx * 16..cx * 16 + 16 {
            for z in cz * 16..cz * 16 + 16 {
                for y in MIN_Y..320 {
                    let (lx, lz) = ((x & 15) as usize, (z & 15) as usize);
                    let blocks = (loaded.block(lx, y, lz), structured.block(lx, y, lz), ours.block(lx, y, lz));
                    let bds = bds_cell(&primary, &a.chunk, x, y, z);
                    let kind = attribute(&areas, (x, y, z));
                    let Some(diff) = stage_diff(blocks, &bds, kind.is_some()) else { continue };
                    if bds != bds_cell(&twin, &b.chunk, x, y, z) {
                        unstable += 1;
                        continue;
                    }
                    let wrote = labels.get(&[x, y, z]).map_or("-".to_string(), |l| l.join(" + "));
                    let first = labels.get(&[x, y, z]).and_then(|l| l.iter().rev().find(|l| *l != "rejoin")).map_or(wrote.as_str(), |l| l.as_str());
                    let (name, raw) = writer(first);
                    let our = format!("{}[{}]", blocks.1.name(), blocks.1.states());
                    let theirs = format!("{}[{}]{}", bds.block, bds.states, bds.liquid.as_deref().map_or(String::new(), |l| format!("+{l}")));
                    let line = format!("{x} {y} {z} {diff:?} {our} | {theirs} | {wrote}");
                    if only.as_deref().is_some_and(|only| !line.contains(only)) {
                        continue;
                    }
                    if shown < show {
                        println!("{line}");
                        shown += 1;
                    }
                    *groups.entry((name, raw, our, theirs)).or_default() += 1;
                    *kinds.entry(format!("{} {diff:?}", kind.map_or("no area", |k| k.name()))).or_default() += 1;
                    *per_chunk.entry((cx, cz)).or_default() += 1;
                }
            }
        }
    });
    let mut sorted: Vec<_> = groups.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let total: u32 = sorted.iter().map(|g| g.1).sum();
    println!("{total} cells differ, {unstable} more where BDS's two runs disagree; by kind: {kinds:?}");
    println!("by chunk: {per_chunk:?}");
    println!("count, writer, template block, ours | BDS:");
    for ((name, raw, ours, theirs), count) in sorted.iter().take(show) {
        println!("{count:>6} {name} | {raw} | {ours} | {theirs}");
    }
}
