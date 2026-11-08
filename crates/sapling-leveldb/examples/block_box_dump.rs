//! Prints every block in a box of a world folder's overworld that is not air, one per line
//! (`x y z name [states]`), skipping columns the folder does not hold:
//! `cargo run -p sapling-leveldb --example block_box_dump -- <world> <x1> <y1> <z1> <x2> <y2> <z2>`.
use std::path::PathBuf;

use sapling_leveldb::LevelDbWorld;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [folder, rest @ ..] = &args[..] else { return Err("usage: block_box_dump <world> <x1> <y1> <z1> <x2> <y2> <z2>".into()) };
    let corners: Vec<i32> = rest.iter().map(|arg| arg.parse()).collect::<Result<_, _>>()?;
    let [x1, y1, z1, x2, y2, z2] = corners[..] else { return Err("six corner coordinates".into()) };
    let ([x1, x2], [y1, y2], [z1, z2]) = ([x1.min(x2), x1.max(x2)], [y1.min(y2), y1.max(y2)], [z1.min(z2), z1.max(z2)]);
    let world = LevelDbWorld::open_in_memory(&PathBuf::from(folder))?;
    let mut keys: Vec<_> = world.columns()?.into_iter().filter(|key| key.dimension == 0 && (x1 >> 4..=x2 >> 4).contains(&key.x) && (z1 >> 4..=z2 >> 4).contains(&key.z)).collect();
    keys.sort_by_key(|key| (key.x, key.z));
    for key in keys {
        let Some(column) = world.load_column(key)? else { continue };
        for x in (key.x * 16).max(x1)..=(key.x * 16 + 15).min(x2) {
            for z in (key.z * 16).max(z1)..=(key.z * 16 + 15).min(z2) {
                for y in y1..=y2 {
                    let Some(state) = world.registry().get(column.chunk.block(x & 15, y, z & 15)) else { continue };
                    if state.name != "minecraft:air" {
                        println!("{x} {y} {z} {} {:?}", state.name, state.properties);
                    }
                }
            }
        }
    }
    Ok(())
}
