//! Lists a world folder's keys that are not chunk records, and prints the named ones' NBT
//! (`portals`, `player_server_<id>`, `~local_player`):
//! `cargo run -p sapling-leveldb --example records -- <world> [key prefix...]`.
use std::path::PathBuf;

use acacia_proto::nbt::{self, LittleEndian};
use sapling_leveldb::{ChunkKey, LevelDbWorld};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let folder = PathBuf::from(args.next().ok_or("usage: records <world folder> [key prefix...]")?);
    let wanted: Vec<String> = args.collect();
    let world = LevelDbWorld::open_in_memory(&folder)?;
    world.db().for_each(|key, value| {
        if ChunkKey::parse(key).is_some() || key.starts_with(b"digp") || key.starts_with(b"actorprefix") {
            return;
        }
        let name = String::from_utf8_lossy(key);
        println!("{name}: {} bytes", value.len());
        if wanted.iter().any(|prefix| name.starts_with(prefix.as_str())) {
            match nbt::read::<LittleEndian>(&mut &value[..]) {
                Ok(tag) => println!("{:#?}", tag.value),
                Err(error) => println!("  not NBT: {error}"),
            }
        }
    })?;
    Ok(())
}
