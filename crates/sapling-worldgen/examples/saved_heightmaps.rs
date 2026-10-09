//! `saved_heightmaps <world>`: per block name, how often it is the cell under BDS's saved heightmap
//! (`top`) and how often it sits above it (`skipped`), over every saved column of an oracle world.
//! Names what `recomputeHeightMap` counts (docs/research/worldgen-bds-features.md, "Heightmaps").
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{bds_cell, open};

const MIN_Y: i32 = -64;

fn main() {
    let world = std::env::args().nth(1).expect("usage: saved_heightmaps <world>");
    let (primary, _) = open(&world).expect("oracle world");
    let mut counts: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for key in primary.columns().unwrap() {
        let Some(column) = primary.load_column(key).unwrap() else { continue };
        let Some(data) = &column.data3d else { continue };
        let (cx, cz) = (key.x * 16, key.z * 16);
        for i in 0..256 {
            let (x, z) = (cx + (i & 15), cz + (i >> 4));
            let top = MIN_Y + i32::from(data.heightmap[i as usize]) - 1;
            for y in (top.max(MIN_Y)..320).rev() {
                let cell = bds_cell(&primary, &column.chunk, x, y, z);
                if cell.is_empty() && cell.liquid.is_none() {
                    continue;
                }
                let name = cell.liquid.map_or(cell.block.clone(), |l| format!("{}+{l}", cell.block));
                let entry = counts.entry(name).or_default();
                if y == top { entry.0 += 1 } else { entry.1 += 1 }
            }
        }
    }
    println!("{:>9} {:>9}  block", "top", "skipped");
    for (block, (top, skipped)) in counts {
        println!("{top:>9} {skipped:>9}  {block}");
    }
}
