//! `city_sculk <seed> <world> <x0> <z0> <x1> <z1> [LIST]`: over a chunk box, the cells where either
//! side has sculk and the other a block of an ancient city (deepslate bricks, tiles, ...), split by
//! which of our stages wrote the sculk and whether BDS's twin run agrees. With `LIST`, the cells.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::{Block, OverworldGenerator};

const CITY: &[&str] = &["deepslate_bricks", "deepslate_tiles", "cracked_deepslate_bricks", "cracked_deepslate_tiles", "polished_deepslate", "cobbled_deepslate"];

fn class(name: &str) -> &'static str {
    match name {
        "sculk" => "sculk",
        _ if CITY.contains(&name) => "city",
        "air" => "air",
        "deepslate" => "deepslate",
        _ if name.contains("sculk") => "sculk-other",
        _ => "other",
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, x0, z0, x1, z1, rest @ ..] = &args[..] else { panic!("usage: city_sculk <seed> <world> <x0> <z0> <x1> <z1> [LIST]") };
    let [x0, z0, x1, z1] = [x0, z0, x1, z1].map(|v| v.parse::<i32>().unwrap());
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, twin) = open(world).expect("oracle world");
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut per_chunk: BTreeMap<(i32, i32), [u32; 4]> = BTreeMap::new();
    for cx in x0..=x1 {
        for cz in z0..=z1 {
            let key = ChunkKey::new(cx, cz, 0);
            let a = primary.load_column(key).unwrap().expect("chunk saved").chunk;
            let b = twin.load_column(key).unwrap().expect("twin saved").chunk;
            let mut carved = generator.surfaced_blocks(cx, cz);
            generator.carve(&mut carved, cx, cz);
            let mut structured = carved.clone();
            generator.place_structures(&mut structured, cx, cz);
            let mut ours = structured.clone();
            generator.decorate(&mut ours, cx, cz);
            generator.place_features(&mut ours, cx, cz);
            for z in 0..16 {
                for x in 0..16 {
                    for y in -64..=64 {
                        let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                        let (c, s, o) = (carved.block(x, y, z), structured.block(x, y, z), ours.block(x, y, z));
                        let bds = bds_cell(&primary, &a, wx, y, wz);
                        let run2 = bds_cell(&twin, &b, wx, y, wz);
                        let (cs, co, cb) = (class(s.name()), class(o.name()), class(&bds.block));
                        for (who, name) in [("ours", o.name()), ("bds", bds.block.as_str()), ("twin", run2.block.as_str()), ("ours structured", s.name())] {
                            if name.contains("sculk") {
                                *counts.entry(format!("total {name} in {who}")).or_default() += 1;
                            }
                        }
                        if (1..15).contains(&x) && (1..15).contains(&z) && y > -64 {
                            let around = [(1i32, 0i32, 0i32), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)];
                            let beside = |name: &dyn Fn(i32, i32, i32) -> String| {
                                let names: Vec<String> = around.iter().map(|&(dx, dy, dz)| name(dx, dy, dz)).collect();
                                if names.iter().any(|n| n == "sculk") {
                                    "beside sculk"
                                } else if names.iter().any(|n| CITY.contains(&n.as_str())) {
                                    "on city blocks"
                                } else {
                                    "on terrain"
                                }
                            };
                            if o.name() == "sculk_vein" {
                                let at = |dx: i32, dy: i32, dz: i32| ours.block((x as i32 + dx) as usize, y + dy, (z as i32 + dz) as usize).name().to_string();
                                *counts.entry(format!("vein {} in ours", beside(&at))).or_default() += 1;
                            }
                            for (who, world, chunk, cell) in [("bds", &primary, &a, &bds), ("twin", &twin, &b, &run2)] {
                                if cell.block == "sculk_vein" {
                                    let at = |dx: i32, dy: i32, dz: i32| bds_cell(world, chunk, wx + dx, y + dy, wz + dz).block;
                                    *counts.entry(format!("vein {} in {who}", beside(&at))).or_default() += 1;
                                }
                            }
                        }
                        if cs == "city" {
                            let twin_class = class(&run2.block);
                            let mark = |c: &str| if c == "sculk" { 'S' } else { '-' };
                            *counts.entry(format!("city cell sculk in ours/bds/twin: {}{}{}", mark(co), mark(cb), mark(twin_class))).or_default() += 1;
                        }
                        let touched = c != s || CITY.contains(&bds.block.as_str());
                        if !touched || !(co == "sculk" || cb == "sculk") {
                            continue;
                        }
                        if co == cb {
                            *counts.entry(format!("both sculk: carved {:<9} structured {cs}", class(c.name()))).or_default() += 1;
                            continue;
                        }
                        let stable = if bds == run2 { "stable" } else { "unstable" };
                        let key = format!("carved {:<9} structured {:<9} final {:<9} bds {:<9} twin {:<9} {stable}", class(c.name()), cs, co, cb, class(&run2.block));
                        *counts.entry(key).or_default() += 1;
                        let slot = match (cs, co, cb) {
                            ("city", "sculk", "city") => 0,
                            ("sculk", "sculk", "city") => 1,
                            ("city", "city", "sculk") => 2,
                            _ => 3,
                        };
                        per_chunk.entry((cx, cz)).or_default()[slot] += 1;
                        if !rest.is_empty() {
                            println!("{wx} {y} {wz} carved {} structured {} final {} bds {} twin {}", c.name(), s.name(), o.name(), bds.block, run2.block);
                        }
                    }
                }
            }
        }
    }
    for (key, n) in counts {
        println!("{n:>6}  {key}");
    }
    println!("per chunk: ours features over city, ours structure sculk over bds city, bds sculk over our city, other");
    for (chunk, n) in per_chunk {
        println!("  {chunk:?} {n:?}");
    }
    let _ = Block::Air;
}
