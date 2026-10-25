//! Prints every block entity a world folder holds, with its block, one per line, then a count per `id`:
//! `cargo run -p sapling-leveldb --example block_entity_dump -- <world> [id...]` (only those ids when given).
use std::collections::BTreeMap;
use std::path::PathBuf;

use sapling_leveldb::{LevelDbWorld, block_entities};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let folder = PathBuf::from(args.next().ok_or("usage: block_entity_dump <world folder> [id...]")?);
    let wanted: Vec<String> = args.collect();
    let world = LevelDbWorld::open_in_memory(&folder)?;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for key in world.columns()? {
        let Some(column) = world.load_column(key)? else { continue };
        for entity in block_entities(&column.block_entities)? {
            let id = format!("{:?}", entity.nbt.get("id"));
            *counts.entry(id.clone()).or_default() += 1;
            if !wanted.is_empty() && !wanted.iter().any(|w| id.contains(w.as_str())) {
                continue;
            }
            let [x, y, z] = entity.pos;
            let block = world.registry().get(column.chunk.block(x & 15, y, z & 15)).map(|state| format!("{} {:?}", state.name, state.properties));
            println!("dim {} {:?} {block:?} {:?}", key.dimension, entity.pos, entity.nbt);
        }
    }
    println!("{counts:#?}");
    Ok(())
}
