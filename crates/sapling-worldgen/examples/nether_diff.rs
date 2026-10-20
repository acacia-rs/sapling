//! `nether_diff <seed> <world dir> <x0> <z0> <x1> <z1> [stage]`: a Nether region against a BDS-saved world, cell by
//! cell (y 0..128) and column by column for biomes, at stage `shape`, `bedrock`, `surfaced` or `carved` (the default).
//! Prints the differing (ours, BDS) block pairs by count, then the first differing cells.
use std::collections::BTreeMap;
use std::path::Path;

use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_worldgen::NetherGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, x0, z0, x1, z1, rest @ ..] = &args[..] else { panic!("usage: nether_diff <seed> <world> <x0> <z0> <x1> <z1> [stage]") };
    let seed: i64 = seed.parse().unwrap();
    let [x0, z0, x1, z1] = [x0, z0, x1, z1].map(|v| v.parse::<i32>().unwrap());
    let stage = rest.first().map_or("carved", String::as_str);
    let world = LevelDbWorld::open_in_memory(Path::new(world)).unwrap();
    let registry = world.registry();
    let generator = NetherGenerator::new(seed);
    let (mut cells, mut pairs, mut first) = (0usize, BTreeMap::<(String, String), usize>::new(), Vec::new());
    let (mut columns, mut biome_pairs) = (0usize, BTreeMap::<(u32, u32), usize>::new());
    for cx in x0..=x1 {
        for cz in z0..=z1 {
            let chunk = world.load_column(ChunkKey::new(cx, cz, 1)).unwrap().expect("chunk saved").chunk;
            let shape = match stage {
                "shape" => generator.chunk_shape(cx, cz),
                "bedrock" => generator.bedrock_blocks(cx, cz),
                "surfaced" => generator.surfaced_blocks(cx, cz),
                _ => generator.carved_blocks(cx, cz),
            };
            let biomes = generator.chunk_biomes(cx, cz);
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    columns += 1;
                    let (ours, bds) = (biomes.biome(x, 0, z), chunk.biome(wx, 0, wz).unwrap_or(u32::MAX));
                    if ours != bds {
                        *biome_pairs.entry((ours, bds)).or_default() += 1;
                    }
                    for y in 0..128 {
                        cells += 1;
                        let ours = shape.block(x, y, z).name();
                        let bds = registry.get(chunk.block(wx, y, wz)).map_or("?", |s| s.name.trim_start_matches("minecraft:"));
                        if ours != bds {
                            *pairs.entry((ours.to_string(), bds.to_string())).or_default() += 1;
                            if first.len() < 20 {
                                first.push(format!("{wx} {y} {wz}: {ours}/{bds}"));
                            }
                        }
                    }
                }
            }
        }
    }
    let differing: usize = pairs.values().sum();
    println!("{cells} cells, {differing} differ; {columns} columns, {} biomes differ", biome_pairs.values().sum::<usize>());
    let mut sorted: Vec<_> = pairs.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    for ((ours, bds), count) in sorted.iter().take(25) {
        println!("{count:>9}  {ours} / {bds}");
    }
    for ((ours, bds), count) in &biome_pairs {
        println!("biome {ours} / {bds}: {count}");
    }
    first.iter().for_each(|line| println!("{line}"));
}
