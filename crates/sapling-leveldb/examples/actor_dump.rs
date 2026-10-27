//! Prints one stored actor of each kind in full, every one with `--all`, and the block entities
//! whose `id` is one of the names after `--blocks`:
//! `cargo run -p sapling-leveldb --example actor_dump -- <world> [--all] [--blocks Beehive ...]`.
use std::collections::BTreeSet;
use std::path::PathBuf;

use acacia_proto::nbt::{self, LittleEndian, Value};
use sapling_leveldb::LevelDbWorld;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let folder = PathBuf::from(args.next().ok_or("usage: actor_dump <world folder> [--all] [--blocks <id>...]")?);
    let rest: Vec<String> = args.collect();
    let all = rest.iter().any(|arg| arg == "--all");
    let blocks: Vec<&str> = rest.iter().skip_while(|arg| *arg != "--blocks").skip(1).map(String::as_str).collect();
    let world = LevelDbWorld::open_in_memory(&folder)?;
    let mut seen = BTreeSet::new();
    for key in world.columns()? {
        let column = world.load_column(key)?.ok_or("a listed column is missing")?;
        let mut rest = &column.actors[..];
        while !rest.is_empty() {
            let actor = nbt::read::<LittleEndian>(&mut rest)?.value;
            if let Some(Value::String(kind)) = actor.get("identifier") {
                if all || seen.insert(kind.clone()) {
                    println!("{actor:?}\n");
                }
            }
        }
        let mut rest = &column.block_entities[..];
        while !rest.is_empty() && !blocks.is_empty() {
            let entity = nbt::read::<LittleEndian>(&mut rest)?.value;
            if matches!(entity.get("id"), Some(Value::String(id)) if blocks.iter().any(|block| id == *block)) {
                println!("{entity:?}\n");
            }
        }
    }
    Ok(())
}
