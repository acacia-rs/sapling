//! Prints one stored actor of each kind in full: `cargo run -p sapling-leveldb --example actor_dump -- <world>`.
use std::collections::BTreeSet;
use std::path::PathBuf;

use acacia_proto::nbt::{self, LittleEndian, Value};
use sapling_leveldb::LevelDbWorld;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder = PathBuf::from(std::env::args().nth(1).ok_or("usage: actor_dump <world folder>")?);
    let world = LevelDbWorld::open_in_memory(&folder)?;
    let mut seen = BTreeSet::new();
    for key in world.columns()? {
        let record = world.load_column(key)?.ok_or("a listed column is missing")?.actors;
        let mut rest = &record[..];
        while !rest.is_empty() {
            let actor = nbt::read::<LittleEndian>(&mut rest)?.value;
            if let Some(Value::String(kind)) = actor.get("identifier") {
                if seen.insert(kind.clone()) {
                    println!("{actor:?}\n");
                }
            }
        }
    }
    Ok(())
}
