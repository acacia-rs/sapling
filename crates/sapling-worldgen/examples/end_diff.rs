//! `end_diff <seed> <world dir> <cx0> <cz0> <cx1> <cz1>`: our finished End chunks against a saved BDS End world,
//! by (ours, BDS) block pair, then the chunks that differ and the first differing cells.

use std::collections::BTreeMap;
use std::path::Path;

use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_worldgen::EndGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: end_diff <seed> <world> cx0 cz0 cx1 cz1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let generator = EndGenerator::new(seed.parse().unwrap());
    let world = LevelDbWorld::open_in_memory(Path::new(world)).expect("world");
    let registry = world.registry();
    let name = |id: u32| registry.get(id).map_or("?".to_string(), |s| format!("{}[{}]", s.name.trim_start_matches("minecraft:"), s.properties));
    let (mut pairs, mut chunks, mut first) = (BTreeMap::<(String, String), usize>::new(), Vec::new(), Vec::new());
    for cx in n[0]..=n[2] {
        for cz in n[1]..=n[3] {
            let saved = world.load_column(ChunkKey::new(cx, cz, 2)).unwrap().expect("chunk saved").chunk;
            let ours = generator.chunk_blocks(cx, cz);
            let mut differing = 0;
            for (x, z, y) in (0..16).flat_map(|x| (0..16).flat_map(move |z| (0..256).map(move |y| (x, z, y)))) {
                let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                let block = ours.block(x, y, z);
                let ours = format!("{}[{}]", block.name(), block.states());
                let theirs = name(saved.block(wx, y, wz));
                if ours != theirs {
                    differing += 1;
                    if first.len() < 30 {
                        first.push(format!("{wx} {y} {wz}: {ours} / {theirs}"));
                    }
                    *pairs.entry((ours, theirs)).or_default() += 1;
                }
            }
            if differing > 0 {
                chunks.push(format!("{cx},{cz}:{differing}"));
            }
        }
    }
    let mut sorted: Vec<_> = pairs.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    println!("{} chunks differ, {} cells", chunks.len(), sorted.iter().map(|p| p.1).sum::<usize>());
    for ((ours, theirs), count) in sorted.iter().take(30) {
        println!("{count:>9}  {ours} / {theirs}");
    }
    println!("chunks: {}", chunks.iter().take(40).cloned().collect::<Vec<_>>().join(" "));
    first.iter().for_each(|line| println!("{line}"));
}
