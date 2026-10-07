//! `shape_diff <seed> <world> (<x0> <z0> <x1> <z1>)...`: per region, connected groups of stable
//! cells where our terrain and BDS disagree on solid, with the nearest BDS block no terrain stage
//! places (structures, features) and its distance. Default: our shape before carvers leaves the
//! cell open, BDS has terrain there. `MODE=carve`: our terrain after carvers is solid, BDS emptied it.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::{HashMap, HashSet};

use oracle_support::{BdsCell, TERRAIN_BLOCKS, bds_cell, open};
use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_worldgen::{CellKind, OverworldGenerator};

const SEARCH: i32 = 12;
type Pos = (i32, i32, i32);

fn foreign(cell: &BdsCell) -> bool {
    !cell.is_empty() && !cell.is_fluid() && !TERRAIN_BLOCKS.contains(&cell.block.as_str())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: shape_diff <seed> <world> (x0 z0 x1 z1)...") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, twin) = open(world).expect("oracle world");
    let carve = std::env::var("MODE").is_ok_and(|m| m == "carve");
    for region in n.chunks(4) {
        println!("region {region:?}");
        report(&collect(&generator, &primary, &twin, region, carve));
    }
}

fn collect(generator: &OverworldGenerator, primary: &LevelDbWorld, twin: &LevelDbWorld, n: &[i32], carve: bool) -> (HashMap<Pos, String>, Vec<(Pos, String)>) {
    let mut cells = HashMap::new();
    let mut strange = Vec::new();
    for cx in n[0] - 1..=n[2] + 1 {
        for cz in n[1] - 1..=n[3] + 1 {
            let key = ChunkKey::new(cx, cz, 0);
            let Some(a) = primary.load_column(key).unwrap().map(|c| c.chunk) else { continue };
            let inside = (n[0]..=n[2]).contains(&cx) && (n[1]..=n[3]).contains(&cz);
            let ours = inside.then(|| if carve { generator.loaded_blocks(cx, cz) } else { generator.surfaced_blocks(cx, cz) });
            let b = inside.then(|| twin.load_column(key).unwrap().expect("twin").chunk);
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    for y in -64..256 {
                        let bds = bds_cell(primary, &a, wx, y, wz);
                        if foreign(&bds) {
                            strange.push(((wx, y, wz), bds.block.clone()));
                        }
                        let (Some(ours), Some(b)) = (&ours, &b) else { continue };
                        let block = ours.block(x, y, z);
                        let differs = if carve {
                            block.is_solid() && (bds.is_empty() || bds.is_fluid())
                        } else {
                            !block.is_solid() && bds.is_terrain() && bds.kind() == CellKind::Solid
                        };
                        if differs && bds == bds_cell(twin, b, wx, y, wz) {
                            cells.insert((wx, y, wz), format!("{block:?}>{}", bds.block));
                        }
                    }
                }
            }
        }
    }
    (cells, strange)
}

fn report((cells, strange): &(HashMap<Pos, String>, Vec<(Pos, String)>)) {
    let mut seen = HashSet::new();
    let mut groups = Vec::new();
    for &start in cells.keys() {
        if !seen.insert(start) {
            continue;
        }
        let (mut stack, mut group) = (vec![start], Vec::new());
        while let Some(p) = stack.pop() {
            group.push(p);
            for d in [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
                let q = (p.0 + d.0, p.1 + d.1, p.2 + d.2);
                if cells.contains_key(&q) && seen.insert(q) {
                    stack.push(q);
                }
            }
        }
        groups.push(group);
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.len()));
    println!("  {} cells in {} groups", cells.len(), groups.len());
    for group in groups.iter().take(12) {
        let lo = group.iter().fold((i32::MAX, i32::MAX, i32::MAX), |l, p| (l.0.min(p.0), l.1.min(p.1), l.2.min(p.2)));
        let hi = group.iter().fold((i32::MIN, i32::MIN, i32::MIN), |h, p| (h.0.max(p.0), h.1.max(p.1), h.2.max(p.2)));
        let within = |q: &Pos| (lo.0 - SEARCH..=hi.0 + SEARCH).contains(&q.0) && (lo.1 - SEARCH..=hi.1 + SEARCH).contains(&q.1) && (lo.2 - SEARCH..=hi.2 + SEARCH).contains(&q.2);
        let distance = |q: &Pos| group.iter().map(|p| (p.0 - q.0).abs().max((p.1 - q.1).abs()).max((p.2 - q.2).abs())).min().unwrap();
        let near = strange.iter().filter(|(q, _)| within(q)).map(|(q, name)| (distance(q), name)).min_by_key(|(d, _)| *d);
        println!("  {:>5} cells {lo:?}..{hi:?} e.g. {} nearest foreign {near:?}", group.len(), cells[&group[0]]);
    }
}
