//! `slice <seed> <world> <x0> <y0> <z0> <x1> <y1> <z1> [six more...]`: per y, ours and BDS side by side, x across,
//! z down: `#` solid, `.` air, `W`/`L` water/lava sources, a hex digit a flowing depth (lava in
//! lower case: `a`-`p` for 0-15 is not used; lava depths print as `l`).
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::HashMap;

use oracle_support::{BdsCell, bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{Block, ChunkShape, OverworldGenerator};

/// `MARK=name:c,...` prints those blocks as their own characters.
fn mark(name: &str) -> Option<char> {
    let marks = std::env::var("MARK").ok()?;
    marks.split(',').find_map(|m| m.split_once(':').filter(|(n, _)| *n == name).and_then(|(_, c)| c.chars().next()))
}

fn ours_char(block: Block) -> char {
    if let Some(c) = mark(block.name()) {
        return c;
    }
    match block.liquid_depth() {
        Some(0) if block.is_lava() => 'L',
        Some(_) if block.is_lava() => 'l',
        Some(0) => 'W',
        Some(d) => char::from_digit(d as u32, 16).unwrap(),
        None if block == Block::Air => '.',
        None if block.is_waterlogged() => 'W',
        None => '#',
    }
}

fn bds_char(cell: &BdsCell) -> char {
    if let Some(c) = mark(&cell.block) {
        return c;
    }
    let depth: Option<u32> = cell.states.strip_prefix("liquid_depth=").and_then(|d| d.parse().ok());
    match (cell.block.as_str(), depth) {
        ("lava" | "flowing_lava", Some(0)) => 'L',
        ("lava" | "flowing_lava", _) => 'l',
        ("water" | "flowing_water", Some(0)) => 'W',
        ("water" | "flowing_water", Some(d)) => char::from_digit(d, 16).unwrap(),
        _ if cell.is_empty() => '.',
        _ if cell.liquid.is_some() => 'W',
        _ => '#',
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: slice <seed> <world> x0 y0 z0 x1 y1 z1") };
    let all: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let mut generator = OverworldGenerator::new(seed.parse().unwrap());
    oracle_support::replay(&mut generator);
    let (primary, _) = open(world).expect("oracle world");
    let mut ours: HashMap<(i32, i32), ChunkShape> = HashMap::new();
    let mut theirs = HashMap::new();
    for (n, y) in all.chunks_exact(6).flat_map(|n| (n[1]..=n[4]).rev().map(move |y| (n, y))) {
        println!("y {y} x {}..{}", n[0], n[3]);
        for z in n[2]..=n[5] {
            let (mut a, mut b) = (String::new(), String::new());
            for x in n[0]..=n[3] {
                let key = (x >> 4, z >> 4);
                let shape = ours.entry(key).or_insert_with(|| generator.chunk_blocks(key.0, key.1));
                a.push(ours_char(shape.block((x & 15) as usize, y, (z & 15) as usize)));
                let chunk = theirs.entry(key).or_insert_with(|| primary.load_column(ChunkKey::new(key.0, key.1, 0)).unwrap().map(|c| c.chunk));
                b.push(chunk.as_ref().map_or('?', |chunk| bds_char(&bds_cell(&primary, chunk, x, y, z))));
            }
            println!("  {a}   {b}   z {z}");
        }
    }
}
