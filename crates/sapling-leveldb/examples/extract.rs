//! Copies the overworld columns around a world's spawn into a new, small world folder; this is
//! how `tests/fixtures/world` was made (tests/fixtures/README.md).
//! `cargo run -p sapling-leveldb --example extract -- <world> <new world> <chunk radius>`
use std::path::PathBuf;

use sapling_leveldb::{Batch, ChunkKey, Db, LevelDbWorld};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [from, to, radius] = &args[..] else { return Err("usage: extract <world> <new world> <chunk radius>".into()) };
    let (from, to, radius) = (PathBuf::from(from), PathBuf::from(to), radius.parse::<i32>()?);

    let world = LevelDbWorld::open_in_memory(&from)?;
    let [x, _, z] = world.level().spawn().ok_or("level.dat has no spawn")?;
    let (cx, cz) = (x >> 4, z >> 4);
    let near = |chunk: ChunkKey| chunk.dimension == 0 && (chunk.x - cx).abs() <= radius && (chunk.z - cz).abs() <= radius;

    let mut batch = Batch::default();
    let mut records = 0;
    world.db().for_each(|key, value| {
        if ChunkKey::parse(key).is_some_and(|parsed| near(parsed.chunk)) {
            batch.put(key, value);
            records += 1;
        }
    })?;
    std::fs::create_dir_all(to.join("db"))?;
    let db = Db::create(&to.join("db"))?;
    db.write(batch)?;
    db.flush()?;
    std::fs::copy(from.join("level.dat"), to.join("level.dat"))?;
    println!("copied {records} records around chunk ({cx}, {cz})");
    Ok(())
}
