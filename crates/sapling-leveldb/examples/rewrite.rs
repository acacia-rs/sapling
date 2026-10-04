//! Copies a world folder and writes every overworld column of the copy back through
//! `LevelDbWorld::store_column`, with a marker in each: the world a vanilla server is then started
//! on to see whether it reads what sapling writes (`bds_world_check` in the sapling crate).
//! `cargo run -p sapling-leveldb --example rewrite -- <world> <new world>`
//!
//! Generated terrain cannot tell a column the game read from one it threw away and generated
//! again, so each column gets blocks no generator places, at local (8, 200.., 8): a diamond block,
//! waterlogged stairs sealed in glass (two layers; a byte and an int state), a log (a string
//! state) and a composter (an int state), none of them in its block's default state.
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use acacia_world::BlockRegistry;
use sapling_leveldb::LevelDbWorld;

const MARKER_Y: i32 = 200;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [from, to] = &args[..] else { return Err("usage: rewrite <world> <new world>".into()) };
    let (from, to) = (PathBuf::from(from), PathBuf::from(to));
    std::fs::create_dir_all(to.join("db"))?;
    for entry in std::fs::read_dir(from.join("db"))? {
        let file = entry?.file_name();
        std::fs::copy(from.join("db").join(&file), to.join("db").join(&file))?;
    }
    std::fs::copy(from.join("level.dat"), to.join("level.dat"))?;

    let registry = BlockRegistry::vanilla();
    let last = |name: &str| registry.states_of(name).last().map(|(id, _)| id).ok_or(format!("no block {name}"));
    // The game recomputes a stair's corner from its neighbours: alone, anything but `none` is corrected.
    let stairs = registry.find("minecraft:oak_stairs", "minecraft:corner=none,upside_down_bit=1,weirdo_direction=3").ok_or("no such stairs")?;
    let (diamond, log, composter) = (last("minecraft:diamond_block")?, last("minecraft:oak_log")?, last("minecraft:composter")?);
    let (glass, water) = (last("minecraft:glass")?, registry.find("minecraft:water", "liquid_depth=0").ok_or("no water source")?);

    let world = LevelDbWorld::open(&to)?;
    let columns: Vec<_> = world.columns()?.into_iter().filter(|key| key.dimension == 0).collect();
    for key in &columns {
        let mut column = world.load_column(*key)?.ok_or("a listed column")?;
        let (x, z) = (key.x * 16 + 8, key.z * 16 + 8);
        for (dy, id) in [diamond, stairs, log, composter].into_iter().enumerate() {
            column.chunk.set(x, MARKER_Y + dy as i32, z, 0, id);
        }
        column.chunk.set(x, MARKER_Y + 1, z, 1, water);
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            column.chunk.set(x + dx, MARKER_Y + 1, z + dz, 0, glass);
        }
        world.store_column(*key, &column.chunk, &column.block_entities, &column.actors)?;
    }
    let mut level = world.level().clone();
    level.set_last_played(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64);
    world.flush(&level)?;
    println!("rewrote {} columns into {}", columns.len(), to.display());
    Ok(())
}
