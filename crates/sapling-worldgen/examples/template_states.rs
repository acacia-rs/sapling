//! `template_states <seed> <world> <x0> <z0> <x1> <z1> [name part]`: over a chunk box, each v2
//! template block our structure stage left in place against BDS's cell, tallied by (unturned
//! block, piece rotation): the pairs where BDS's block is not ours show a state the rotation
//! table turns wrong. A name part keeps only the blocks whose name contains it and lists the cells.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, x0, z0, x1, z1, rest @ ..] = &args[..] else { panic!("usage: template_states <seed> <world> <x0> <z0> <x1> <z1> [name part]") };
    let [x0, z0, x1, z1] = [x0, z0, x1, z1].map(|v| v.parse::<i32>().unwrap());
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    let mut tally: BTreeMap<(String, u8, String), BTreeMap<String, u32>> = BTreeMap::new();
    for cx in x0..=x1 {
        for cz in z0..=z1 {
            let cells = generator.v2_template_cells(cx, cz);
            if cells.is_empty() {
                continue;
            }
            let Some(column) = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap() else { continue };
            let mut ours = generator.surfaced_blocks(cx, cz);
            generator.carve(&mut ours, cx, cz);
            generator.place_structures(&mut ours, cx, cz);
            for ([x, y, z], template, unturned, rotation, turned) in cells {
                if ours.block((x & 15) as usize, y, (z & 15) as usize) != turned || rest.first().is_some_and(|part| !turned.name().contains(part.as_str())) {
                    continue;
                }
                let bds = bds_cell(&primary, &column.chunk, x, y, z);
                if bds.block != turned.name() {
                    continue;
                }
                let key = (format!("{}[{}]", unturned.name(), unturned.states()), rotation, format!("[{}]", turned.states()));
                *tally.entry(key).or_default().entry(format!("[{}]", bds.states)).or_default() += 1;
                if !rest.is_empty() && bds.states != turned.states() {
                    println!("{x} {y} {z} {template} rotation {rotation}: ours [{}] bds [{}]", turned.states(), bds.states);
                }
            }
        }
    }
    for ((unturned, rotation, ours), seen) in tally {
        if seen.len() > 1 || !seen.contains_key(&ours) || !rest.is_empty() {
            println!("{unturned} rotation {rotation}: ours {ours}, bds {seen:?}");
        }
    }
}
