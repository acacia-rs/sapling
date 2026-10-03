//! Prints what a world folder holds: `cargo run -p sapling-leveldb --example inspect -- <world>`.
use std::collections::BTreeMap;
use std::path::PathBuf;

use sapling_leveldb::{ChunkKey, LevelDbWorld};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder = PathBuf::from(std::env::args().nth(1).ok_or("usage: inspect <world folder>")?);
    let world = LevelDbWorld::open_in_memory(&folder)?;
    let level = world.level();
    println!("{:?}: storage version {}, seed {:?}, spawn {:?}", level.name(), level.storage_version, level.seed(), level.spawn());

    let mut tags: BTreeMap<u8, usize> = BTreeMap::new();
    let mut other = 0;
    world.db().for_each(|key, _| match ChunkKey::parse(key) {
        Some(parsed) => *tags.entry(parsed.tag).or_default() += 1,
        None => other += 1,
    })?;
    println!("chunk records by tag: {tags:#04x?}; {other} other keys");

    let columns = world.columns()?;
    let (xs, zs) = (columns.iter().map(|c| c.x), columns.iter().map(|c| c.z));
    println!("{} columns, x {:?}..={:?}, z {:?}..={:?}", columns.len(), xs.clone().min(), xs.max(), zs.clone().min(), zs.max());
    let mut versions: BTreeMap<u8, usize> = BTreeMap::new();
    for &key in &columns {
        let column = world.load_column(key)?.ok_or("a listed column is missing")?;
        *versions.entry(column.version).or_default() += 1;
    }
    println!("chunk versions: {versions:?}");
    Ok(())
}
