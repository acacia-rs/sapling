//! `end_buffers <seed> <buffers dir>`: every End chunk buffer a debugger run dumped as `loadChunk` committed it
//! (scripts/data/dbg_end_buffers.json) against our terrain, block name by block name.
#[path = "../tests/nether_support/mod.rs"]
mod nether_support;

use std::collections::BTreeMap;
use std::path::Path;

use nether_support::{Buffers, index};
use sapling_worldgen::EndGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, dir] = &args[..] else { panic!("usage: end_buffers <seed> <buffers dir>") };
    let generator = EndGenerator::new(seed.parse().unwrap());
    let buffers = Buffers::open_named(Path::new(dir), "end").expect("block_ptrs.tsv");
    let (mut pairs, mut chunks, mut first) = (BTreeMap::<(String, String), usize>::new(), Vec::new(), Vec::new());
    let all = buffers.chunks();
    for &(cx, cz) in &all {
        let bds = buffers.chunk(cx, cz).unwrap();
        let shape = generator.terrain_blocks(cx, cz);
        let mut differing = 0;
        for (x, z, y) in (0..16).flat_map(|x| (0..16).flat_map(move |z| (0..128).map(move |y| (x, z, y)))) {
            let (ours, theirs) = (shape.block(x, y, z).name(), bds[index(x, y, z)]);
            if ours != theirs {
                differing += 1;
                *pairs.entry((ours.to_string(), theirs.to_string())).or_default() += 1;
                if first.len() < 30 {
                    first.push(format!("{} {y} {}: {ours}/{theirs}", cx * 16 + x as i32, cz * 16 + z as i32));
                }
            }
        }
        if differing > 0 {
            chunks.push(format!("{cx},{cz}:{differing}"));
        }
    }
    println!("{} chunks, {} differ, {} cells", all.len(), chunks.len(), pairs.values().sum::<usize>());
    for ((ours, theirs), count) in &pairs {
        println!("{count:>9}  {ours} / {theirs}");
    }
    println!("chunks: {}", chunks.iter().take(40).cloned().collect::<Vec<_>>().join(" "));
    first.iter().for_each(|line| println!("{line}"));
}
