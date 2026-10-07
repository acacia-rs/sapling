//! `fluid_diff <seed> <world> <x0> <z0> <x1> <z1>`: connected groups of stable cells where our
//! generated chunk and BDS disagree and either side is a fluid (liquid flow), largest first, with
//! the block pairs, BDS's liquid depths and whether the group touches a chunk's border columns.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::{BTreeMap, HashMap, HashSet};

use oracle_support::{BdsCell, bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{CellKind, OverworldGenerator};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: fluid_diff <seed> <world> x0 z0 x1 z1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, twin) = open(world).expect("oracle world");
    let mut cells: HashMap<(i32, i32, i32), (String, BdsCell)> = HashMap::new();
    let mut names: BTreeMap<String, u64> = BTreeMap::new();
    for cx in n[0]..=n[2] {
        for cz in n[1]..=n[3] {
            let key = ChunkKey::new(cx, cz, 0);
            let a = primary.load_column(key).unwrap().expect("chunk").chunk;
            let b = twin.load_column(key).unwrap().expect("twin").chunk;
            let ours = generator.chunk_blocks(cx, cz);
            for z in 0..16 {
                for x in 0..16 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    for y in -64..256 {
                        let bds = bds_cell(&primary, &a, wx, y, wz);
                        if bds.is_fluid() {
                            *names.entry(format!("{}[{}]", bds.block, bds.states)).or_default() += 1;
                        }
                        if bds != bds_cell(&twin, &b, wx, y, wz) {
                            continue;
                        }
                        let block = ours.block(x, y, z);
                        let our_kind = if block.is_waterlogged() { CellKind::Water } else { ours.kind(x, y, z) };
                        let fluid_side = bds.is_fluid() || block.is_fluid();
                        let differs = our_kind != bds.kind() || bds.is_fluid() && !bds.is(block);
                        if fluid_side && differs && (bds.is_fluid() || bds.is_empty()) {
                            cells.insert((wx, y, wz), (block.name().to_string(), bds));
                        }
                    }
                }
            }
        }
    }
    println!("BDS fluid cells by state: {names:?}");
    let mut seen = HashSet::new();
    let mut groups = Vec::new();
    for &start in cells.keys() {
        if !seen.insert(start) {
            continue;
        }
        let mut stack = vec![start];
        let mut group = Vec::new();
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
    println!("{} cells in {} groups", cells.len(), groups.len());
    let limit = std::env::var("GROUPS").ok().and_then(|v| v.parse().ok()).unwrap_or(40);
    for group in groups.iter().take(limit) {
        let (mut lo, mut hi) = ((i32::MAX, i32::MAX, i32::MAX), (i32::MIN, i32::MIN, i32::MIN));
        let mut pairs: BTreeMap<String, u64> = BTreeMap::new();
        let mut border = 0;
        for p in group {
            lo = (lo.0.min(p.0), lo.1.min(p.1), lo.2.min(p.2));
            hi = (hi.0.max(p.0), hi.1.max(p.1), hi.2.max(p.2));
            let (ours, bds) = &cells[p];
            *pairs.entry(format!("{ours}>{}[{}]", bds.block, bds.states.replace("liquid_depth=", ""))).or_default() += 1;
            if matches!(p.0.rem_euclid(16), 0 | 15) || matches!(p.2.rem_euclid(16), 0 | 15) {
                border += 1;
            }
        }
        let mut pairs: Vec<_> = pairs.into_iter().collect();
        pairs.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        let pairs: Vec<String> = pairs.iter().take(6).map(|(k, n)| format!("{k}:{n}")).collect();
        println!("{:>5} cells {lo:?}..{hi:?} border {border} {}", group.len(), pairs.join(" "));
    }
}
