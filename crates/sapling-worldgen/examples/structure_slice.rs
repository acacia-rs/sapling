//! `structure_slice <seed> <world> <x0> <z0> <x1> <z1> <y0> <y1>`: per layer, our blocks after the
//! structure stage beside BDS's, one letter per block name (legend after each layer; `.` air,
//! `~` water, lowercase where the two disagree), for seeing how a placed structure is offset;
//! with `DIFF` set, one line per differing cell instead (`SAME` adds our non-air matches);
//! `BDSDUMP` lists BDS's non-air cells.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, rest @ ..] = &args[..] else { panic!("usage: structure_slice <seed> <world> x0 z0 x1 z1 y0 y1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let (x0, z0, x1, z1, y0, y1) = (n[0], n[1], n[2], n[3], n[4], n[5]);
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    let mut ours = BTreeMap::new();
    let mut theirs = BTreeMap::new();
    for cx in x0.div_euclid(16)..=x1.div_euclid(16) {
        for cz in z0.div_euclid(16)..=z1.div_euclid(16) {
            let shape = generator.loaded_blocks(cx, cz);
            let column = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap().expect("saved chunk");
            for x in (cx * 16).max(x0)..=(cx * 16 + 15).min(x1) {
                for z in (cz * 16).max(z0)..=(cz * 16 + 15).min(z1) {
                    for y in y0..=y1 {
                        let b = shape.block((x & 15) as usize, y, (z & 15) as usize);
                        ours.insert((x, y, z), b.name().to_string());
                        theirs.insert((x, y, z), bds_cell(&primary, &column.chunk, x, y, z).block);
                    }
                }
            }
        }
    }
    if std::env::var_os("BDSDUMP").is_some() {
        for (pos, b) in theirs.iter().filter(|(_, b)| b.as_str() != "air") {
            println!("{} {} {} {b}", pos.0, pos.1, pos.2);
        }
        return;
    }
    if std::env::var_os("DIFF").is_some() {
        for (pos, a) in &ours {
            let b = &theirs[pos];
            if a != b || std::env::var_os("SAME").is_some() && a != "air" {
                println!("{} {} {} ours {a} bds {b}", pos.0, pos.1, pos.2);
            }
        }
        return;
    }
    let mut letters: BTreeMap<String, char> = BTreeMap::new();
    let mut letter = |name: &str| -> char {
        match name {
            "air" => '.',
            "water" => '~',
            _ => {
                let next = (b'A' + (letters.len() % 26) as u8) as char;
                *letters.entry(name.to_string()).or_insert(next)
            }
        }
    };
    for y in (y0..=y1).rev() {
        println!("y {y}: ours | BDS (x {x0}..{x1} across, z {z0}..{z1} down)");
        for z in z0..=z1 {
            let mut row = (String::new(), String::new());
            for x in x0..=x1 {
                let (a, b) = (&ours[&(x, y, z)], &theirs[&(x, y, z)]);
                let (ca, cb) = (letter(a), letter(b));
                let same = a == b;
                row.0.push(if same { ca } else { ca.to_ascii_lowercase() });
                row.1.push(if same { cb } else { cb.to_ascii_lowercase() });
            }
            println!("  {} | {}", row.0, row.1);
        }
    }
    let legend: Vec<String> = letters.iter().map(|(name, c)| format!("{c}={name}")).collect();
    println!("legend: {}", legend.join(" "));
}
