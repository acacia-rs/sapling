//! `mineshaft_pieces <seed> <world> <x0> <z0> <x1> <z1> <cx,cz>...`: mineshaft starts' pieces in
//! generation order, each with the cells of its box (one block wider and lower) inside the block
//! area where our blocks or BDS's are mineshaft blocks (wood, rails, webs, torches, chains,
//! spawners) and the two disagree (cells BDS's two runs disagree on skipped), with the first few;
//! then those cells per chunk.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use oracle_support::{bds_cell, open};
use sapling_leveldb::{ChunkKey, DiskColumn};
use sapling_worldgen::{ChunkShape, OverworldGenerator};

const PALETTE: [&str; 9] = ["planks", "fence", "rail", "torch", "web", "chain", "_log", "mob_spawner", "chest"];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, x0, z0, x1, z1, starts @ ..] = &args[..] else { panic!("usage: mineshaft_pieces <seed> <world> x0 z0 x1 z1 cx,cz...") };
    let area: Vec<i32> = [x0, z0, x1, z1].iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, twin) = open(world).expect("oracle world");
    let mut chunks: HashMap<(i32, i32), Option<(ChunkShape, DiskColumn, DiskColumn)>> = HashMap::new();
    let palette = |name: &str| PALETTE.iter().any(|p| name.contains(p));
    let mut wrong_cells = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for start in starts {
        let (cx, cz) = start.split_once(',').expect("cx,cz");
        println!("start {start}");
        for (index, (description, bb)) in generator.mineshaft_pieces(cx.parse().unwrap(), cz.parse().unwrap()).iter().enumerate() {
            let (mut wrong, mut total, mut shown) = (0, 0, Vec::new());
            for x in (bb.x0 - 1).max(area[0])..=(bb.x1 + 1).min(area[2]) {
                for z in (bb.z0 - 1).max(area[1])..=(bb.z1 + 1).min(area[3]) {
                    let key = (x.div_euclid(16), z.div_euclid(16));
                    let entry = chunks.entry(key).or_insert_with(|| {
                        let column = primary.load_column(ChunkKey::new(key.0, key.1, 0)).unwrap()?;
                        let other = twin.load_column(ChunkKey::new(key.0, key.1, 0)).unwrap()?;
                        Some((generator.loaded_blocks(key.0, key.1), column, other))
                    });
                    let Some((ours, column, other)) = entry else { continue };
                    for y in bb.y0 - 1..=bb.y1 {
                        let block = ours.block((x & 15) as usize, y, (z & 15) as usize);
                        let bds = bds_cell(&primary, &column.chunk, x, y, z);
                        if (!palette(block.name()) && !palette(&bds.block)) || bds != bds_cell(&twin, &other.chunk, x, y, z) {
                            continue;
                        }
                        total += 1;
                        seen.insert((key, (x, y, z)));
                        if !bds.is(block) && block.name() != bds.block {
                            wrong += 1;
                            wrong_cells.insert((key, (x, y, z)));
                            if shown.len() < 6 {
                                shown.push(format!("({x},{y},{z}) {} | {}", block.name(), bds.block));
                            }
                        }
                    }
                }
            }
            if total > 0 {
                println!("{index:>3} {description} {:?}: {wrong}/{total} wrong {}", (bb.x0, bb.y0, bb.z0, bb.x1, bb.y1, bb.z1), shown.join("; "));
            }
        }
    }
    let mut per_chunk: BTreeMap<(i32, i32), (u32, u32)> = BTreeMap::new();
    seen.iter().for_each(|(key, _)| per_chunk.entry(*key).or_default().1 += 1);
    wrong_cells.iter().for_each(|(key, _)| per_chunk.entry(*key).or_default().0 += 1);
    for (key, (wrong, total)) in per_chunk {
        println!("chunk {key:?}: {wrong}/{total} wrong");
    }
    println!("total {}/{} wrong", wrong_cells.len(), seen.len());
}
