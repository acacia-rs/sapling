//! `nether_buffers <seed> <buffers dir> [shape|bedrock|surfaced|carved]`: every Nether chunk buffer a debugger run
//! dumped as `loadChunk` committed it (scripts/data/dbg_nether_buffers.json) against that stage of ours, block name
//! by block name. Prints the differing (ours, BDS) pairs by count, the chunks that differ, then the first cells.
#[path = "../tests/nether_support/mod.rs"]
mod nether_support;

use std::collections::BTreeMap;
use std::path::Path;

use nether_support::{Buffers, index};
use sapling_worldgen::NetherGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, dir, rest @ ..] = &args[..] else { panic!("usage: nether_buffers <seed> <buffers dir> [stage]") };
    let stage = rest.first().map_or("carved", String::as_str);
    let generator = NetherGenerator::new(seed.parse().unwrap());
    let buffers = Buffers::open(Path::new(dir)).expect("block_ptrs.tsv");
    let (mut pairs, mut chunks, mut first) = (BTreeMap::<(String, String), usize>::new(), Vec::new(), Vec::new());
    let all = buffers.chunks();
    for &(cx, cz) in &all {
        let bds = buffers.chunk(cx, cz).unwrap();
        let shape = match stage {
            "shape" => generator.chunk_shape(cx, cz),
            "bedrock" => generator.bedrock_blocks(cx, cz),
            "surfaced" => generator.surfaced_blocks(cx, cz),
            _ => generator.carved_blocks(cx, cz),
        };
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
    let mut sorted: Vec<_> = pairs.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    for ((ours, theirs), count) in sorted.iter().take(30) {
        println!("{count:>9}  {ours} / {theirs}");
    }
    println!("chunks: {}", chunks.iter().take(40).cloned().collect::<Vec<_>>().join(" "));
    first.iter().for_each(|line| println!("{line}"));
}
