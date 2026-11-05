//! Lists a world folder's keys that are not chunk records, and prints the named ones' NBT
//! (`portals`, `player_server_<id>`, `~local_player`); the prefix `pending_ticks` prints every
//! column's `PendingTicks` record instead, and `tag=<n>` every column's record of tag n in hex:
//! `cargo run -p sapling-leveldb --example records -- <world> [key prefix...]`.
use std::path::PathBuf;

use acacia_proto::nbt::{self, LittleEndian};
use sapling_leveldb::{ChunkKey, LevelDbWorld, Tag};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let folder = PathBuf::from(args.next().ok_or("usage: records <world folder> [key prefix...]")?);
    let wanted: Vec<String> = args.collect();
    let ticks = wanted.iter().any(|prefix| prefix == "pending_ticks");
    let raw: Option<u8> = wanted.iter().find_map(|prefix| prefix.strip_prefix("tag=")?.parse().ok());
    let world = LevelDbWorld::open_in_memory(&folder)?;
    world.db().for_each(|key, value| {
        let show = |value: &[u8]| match nbt::read::<LittleEndian>(&mut &value[..]) {
            Ok(tag) => println!("{:#?}", tag.value),
            Err(error) => println!("  not NBT: {error}"),
        };
        if let Some(parsed) = ChunkKey::parse(key) {
            if raw == Some(parsed.tag) {
                let hex: Vec<String> = value.iter().map(|byte| format!("{byte:02x}")).collect();
                println!("{:?}: {}", parsed.chunk, hex.join(" "));
            } else if ticks && parsed.tag == Tag::PendingTicks as u8 {
                println!("pending ticks of {:?}: {} bytes", parsed.chunk, value.len());
                show(value);
            }
            return;
        }
        if ticks || raw.is_some() || key.starts_with(b"digp") || key.starts_with(b"actorprefix") {
            return;
        }
        let name = String::from_utf8_lossy(key);
        println!("{name}: {} bytes", value.len());
        if wanted.iter().any(|prefix| name.starts_with(prefix.as_str())) {
            show(value);
        }
    })?;
    Ok(())
}
