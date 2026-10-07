//! `palette <world> <cx0> <cz0> <cx1> <cz1>`: every block state of the oracle chunks with its count
//! and lowest/highest y, to see which names and states BDS's terrain stages write.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;

use oracle_support::open;
use sapling_leveldb::ChunkKey;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [world, x0, z0, x1, z1] = &args[..] else { panic!("usage: palette <world> <cx0> <cz0> <cx1> <cz1>") };
    let [x0, z0, x1, z1]: [i32; 4] = [x0, z0, x1, z1].map(|v| v.parse().unwrap());
    let (primary, _) = open(world).expect("oracle world");
    let registry = primary.registry();
    let mut counts: BTreeMap<u32, (u64, i32, i32)> = BTreeMap::new();
    for cx in x0..=x1 {
        for cz in z0..=z1 {
            let chunk = primary.load_column(ChunkKey::new(cx, cz, 0)).unwrap().expect("chunk saved").chunk;
            for y in -64..320 {
                for i in 0..256 {
                    let id = chunk.block(cx * 16 + (i & 15), y, cz * 16 + (i >> 4));
                    let entry = counts.entry(id).or_insert((0, y, y));
                    entry.0 += 1;
                    entry.2 = y;
                }
            }
        }
    }
    for (id, (count, low, high)) in counts {
        let state = registry.get(id).unwrap();
        println!("{count:>10} y {low:>4}..={high:<4} {} [{}]", state.name, state.properties);
    }
}
