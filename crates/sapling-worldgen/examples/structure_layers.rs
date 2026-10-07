//! `structure_layers <seed> <world> <x0> <y0> <z0> <x1> <y1> <z1>`: BDS's blocks in a box, one
//! y layer at a time (rows z, columns x, two-letter codes with a legend), each followed by our
//! loaded blocks where they differ (`..` where equal); for porting structure pieces.
//! `STAGE=decorated` compares the fully decorated chunk instead.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: structure_layers <seed> <world> x0 y0 z0 x1 y1 z1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let (x0, y0, z0, x1, y1, z1) = (n[0], n[1], n[2], n[3], n[4], n[5]);
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let decorated = std::env::var("STAGE").is_ok_and(|s| s == "decorated");
    let (primary, _) = open(world).expect("oracle world");
    let mut legend: BTreeMap<String, String> = BTreeMap::new();
    let mut code = |name: String| {
        let next = legend.len();
        legend.entry(name).or_insert_with(|| if next == 0 { " .".into() } else { format!("{:>2}", base36(next)) }).clone()
    };
    code("air[]".into());
    let mut chunks = BTreeMap::new();
    for y in (y0..=y1).rev() {
        let (mut theirs, mut ours) = (Vec::new(), Vec::new());
        for z in z0..=z1 {
            let (mut row, mut our_row) = (String::new(), String::new());
            for x in x0..=x1 {
                let (cx, cz) = (x.div_euclid(16), z.div_euclid(16));
                let (column, shape) = chunks.entry((cx, cz)).or_insert_with(|| {
                    let column = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap().expect("chunk saved").chunk;
                    (column, if decorated { generator.chunk_blocks(cx, cz) } else { generator.loaded_blocks(cx, cz) })
                });
                let bds = bds_cell(&primary, column, x, y, z);
                let block = shape.block((x & 15) as usize, y, (z & 15) as usize);
                let liquid = bds.liquid.as_ref().map_or(String::new(), |l| format!("+{l}"));
                row.push_str(&code(format!("{}[{}]{liquid}", bds.block, bds.states)));
                our_row.push_str(&if bds.is(block) { "..".to_string() } else { code(format!("{}[{}]", block.name(), block.states())) });
            }
            theirs.push(row);
            ours.push(our_row);
        }
        println!("y {y}  (BDS | ours where different; rows z {z0}..={z1}, columns x {x0}..={x1})");
        for (t, o) in theirs.iter().zip(&ours) {
            println!("  {t}   {o}");
        }
    }
    let mut sorted: Vec<_> = legend.into_iter().collect();
    sorted.sort_by(|a, b| a.1.cmp(&b.1));
    for (name, code) in sorted {
        println!("{code} {name}");
    }
}

fn base36(mut n: usize) -> String {
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut s = Vec::new();
    loop {
        s.push(digits[n % digits.len()]);
        n /= digits.len();
        if n == 0 {
            break;
        }
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}
