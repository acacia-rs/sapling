//! `tree_diff <seed> <world> <cx0> <cz0> <cx1> <cz1>`: the trunk bases (an upright log over a
//! non-log) of ours and BDS's in the chunks, as `x,y,z species height`, marking the ones only one
//! side has.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{MIN_Y, OverworldGenerator};

type Trunks = BTreeMap<(i32, i32, i32), (String, i32)>;

fn upright_log(name: &str, states: &str) -> bool {
    name.ends_with("_log") && states.contains("pillar_axis=y")
}

fn trunks(column: impl Fn(i32) -> (String, String), (x, z): (i32, i32), out: &mut Trunks) {
    let mut y = MIN_Y + 1;
    while y < 320 {
        let (name, states) = column(y);
        if upright_log(&name, &states) && !upright_log(&column(y - 1).0, &column(y - 1).1) {
            let mut top = y;
            while top + 1 < 320 && column(top + 1).0 == name {
                top += 1;
            }
            out.insert((x, y, z), (name.trim_end_matches("_log").to_string(), top - y + 1));
            y = top;
        }
        y += 1;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: tree_diff <seed> <world> <cx0> <cz0> <cx1> <cz1>") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    let (mut ours, mut bds) = (Trunks::new(), Trunks::new());
    for cx in n[0]..=n[2] {
        for cz in n[1]..=n[3] {
            let shape = generator.chunk_blocks(cx, cz);
            let chunk = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap().expect("chunk").chunk;
            for i in 0..256usize {
                let (x, z) = (i & 15, i >> 4);
                let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                let mine = |y| {
                    let b = shape.block(x, y, z);
                    (b.name().to_string(), b.states().to_string())
                };
                trunks(mine, (wx, wz), &mut ours);
                let theirs = |y| {
                    let c = bds_cell(&primary, &chunk, wx, y, wz);
                    (c.block, c.states)
                };
                trunks(theirs, (wx, wz), &mut bds);
            }
        }
    }
    let mut same = 0;
    for (at, tree) in &bds {
        match ours.get(at) {
            Some(mine) if mine == tree => same += 1,
            Some(mine) => println!("{at:?} bds {tree:?} ours {mine:?}"),
            None => println!("{at:?} bds {tree:?} only"),
        }
    }
    for (at, tree) in &ours {
        if !bds.contains_key(at) {
            println!("{at:?} ours {tree:?} only");
        }
    }
    println!("same {same}, bds {}, ours {}", bds.len(), ours.len());
}
