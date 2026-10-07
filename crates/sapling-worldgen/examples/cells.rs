//! `cells <seed> <world> <x0> <y0> <z0> <x1> <y1> <z1>`: ours and BDS's blocks in a box, one grid
//! per layer (rows z, columns x), top layer first: `.` air, `L` log, `#` leaves, `v` vine, `~`
//! water, `o` other; a differing cell shows `ours/bds` in a list under the grid.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;

fn glyph(name: &str) -> char {
    match name {
        "air" => '.',
        "water" => '~',
        n if n.ends_with("_log") => 'L',
        n if n.ends_with("leaves") => '#',
        "vine" => 'v',
        "mud" => 'm',
        "seagrass" => 's',
        "moss_carpet" => 'c',
        "mangrove_propagule" => 'p',
        "mangrove_roots" => 'r',
        "muddy_mangrove_roots" => 'u',
        _ => 'o',
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: cells <seed> <world> x0 y0 z0 x1 y1 z1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    for y in (n[1]..=n[4]).rev() {
        println!("y {y}  ours | bds");
        let mut diffs = Vec::new();
        for z in n[2]..=n[5] {
            let (mut ours, mut theirs) = (String::new(), String::new());
            for x in n[0]..=n[3] {
                let shape = generator.chunk_blocks(x.div_euclid(16), z.div_euclid(16));
                let chunk = primary.load_column(ChunkKey::new(x.div_euclid(16), z.div_euclid(16), 0)).unwrap().expect("chunk").chunk;
                let mine = shape.block(x.rem_euclid(16) as usize, y, z.rem_euclid(16) as usize);
                let cell = bds_cell(&primary, &chunk, x, y, z);
                ours.push(glyph(mine.name()));
                theirs.push(glyph(&cell.block));
                if !cell.is(mine) {
                    diffs.push(format!("{x},{z}: {}[{}]/{}[{}]", mine.name(), mine.states(), cell.block, cell.states));
                }
            }
            println!("  z {z:>6} {ours} | {theirs}");
        }
        for diff in diffs.iter().filter(|d| !d.contains("air[]/air[]")) {
            println!("    {diff}");
        }
    }
}
