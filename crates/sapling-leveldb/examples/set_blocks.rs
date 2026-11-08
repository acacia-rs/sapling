//! Sets blocks in a world folder while no server has it open, to start a server on a world edited offline:
//! `cargo run -p sapling-leveldb --example set_blocks -- <world> <x>,<y>,<z>=<name>[<k=v,...>] ...`
//! (`[...]` as sapling's registry lists the state, e.g. `0,-60,0=minecraft:scaffolding[stability=7,stability_check=false]`).
//! Each column written keeps its block entities, actors and pending ticks.
use std::collections::BTreeMap;
use std::path::PathBuf;

use sapling_leveldb::{ChunkKey, LevelDbWorld};

fn parse(arg: &str) -> Option<([i32; 3], &str, &str)> {
    let (at, block) = arg.split_once('=')?;
    let at: Vec<i32> = at.split(',').map(str::parse).collect::<Result<_, _>>().ok()?;
    let (name, properties) = block.split_once('[').map_or((block, ""), |(name, rest)| (name, rest.trim_end_matches(']')));
    Some(([*at.first()?, *at.get(1)?, *at.get(2)?], name, properties))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let folder = PathBuf::from(args.next().ok_or("usage: set_blocks <world> <x>,<y>,<z>=<name>[<props>] ...")?);
    let world = LevelDbWorld::open(&folder)?;
    let mut by_column: BTreeMap<(i32, i32), Vec<([i32; 3], u32)>> = BTreeMap::new();
    for arg in args {
        let (at, name, properties) = parse(&arg).ok_or_else(|| format!("unreadable: {arg}"))?;
        let id = world.registry().find(name, properties).ok_or_else(|| format!("no state {name}[{properties}]"))?;
        by_column.entry((at[0] >> 4, at[2] >> 4)).or_default().push((at, id));
    }
    for ((x, z), cells) in by_column {
        let key = ChunkKey { x, z, dimension: 0 };
        let mut column = world.load_column(key)?.ok_or_else(|| format!("no column {x} {z}"))?;
        for ([bx, by, bz], id) in &cells {
            column.chunk.set(bx & 15, *by, bz & 15, 0, *id);
        }
        world.store_column(key, &column.chunk, &column.block_entities, &column.actors, &column.pending_ticks)?;
        println!("column {x} {z}: {} blocks set", cells.len());
    }
    world.flush(world.level())?;
    Ok(())
}
