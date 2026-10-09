//! `saved_chunks <world> <cx0> <cz0> <cx1> <cz1>`: for each chunk of the box (rows z, columns x) what
//! BDS saved in the world and in its `-run2` twin: `.` nothing, else the `FinalizedState` value
//! (0 needs ticking, 1 needs population, 2 done; `?` when the record is absent). Shows which ring
//! chunks around a region each run had decorated when it saved.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use oracle_support::open;
use sapling_leveldb::{ChunkKey, LevelDbWorld, Tag};

fn state(world: &LevelDbWorld, x: i32, z: i32) -> char {
    let key = ChunkKey::new(x, z, 0);
    if world.db().get(&key.tag(Tag::Version)).unwrap().is_none() {
        return '.';
    }
    match world.db().get(&key.tag(Tag::FinalizedState)).unwrap() {
        Some(record) if !record.is_empty() => char::from_digit(u32::from(record[0]).min(9), 10).unwrap(),
        _ => '?',
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [world, rest @ ..] = &args[..] else { panic!("usage: saved_chunks <world> cx0 cz0 cx1 cz1") };
    let n: Vec<i32> = rest.iter().map(|v| v.parse().unwrap()).collect();
    let (primary, twin) = open(world).expect("oracle world");
    for z in n[1]..=n[3] {
        let row = |world: &LevelDbWorld| (n[0]..=n[2]).map(|x| state(world, x, z)).collect::<String>();
        println!("z {z:>6}  {}   {}", row(&primary), row(&twin));
    }
}
