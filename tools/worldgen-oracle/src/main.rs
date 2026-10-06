//! Reads oracle worlds (README.md):
//!   worldgen-oracle <world> <block x> <block z>         one column's blocks, liquids and biome ids by y
//!   worldgen-oracle <world> check <x0,z0,x1,z1>...       every overworld chunk of the regions saved and untouched
//!   worldgen-oracle <world> diff <other> <x0,z0,x1,z1>... blocks and biomes of the regions compared
use std::path::Path;

use acacia_world::{BlockRegistry, Chunk};
use sapling_leveldb::{ChunkKey, LevelDbWorld};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let world = LevelDbWorld::open_in_memory(Path::new(args.first().ok_or("usage: see the file header")?))?;
    match args.get(1).map(String::as_str) {
        Some("check") => check(&world, &regions(&args[2..])?),
        Some("diff") => diff(&world, &LevelDbWorld::open_in_memory(Path::new(args.get(2).ok_or("diff needs a second world")?))?, &regions(&args[3..])?),
        _ => dump(&world, args.get(1).ok_or("missing x")?.parse()?, args.get(2).ok_or("missing z")?.parse()?),
    }
}

fn regions(args: &[String]) -> Result<Vec<ChunkKey>> {
    let mut keys = Vec::new();
    for region in args {
        let parts: Vec<i32> = region.split(',').map(str::parse).collect::<std::result::Result<_, _>>()?;
        let [x0, z0, x1, z1] = parts[..] else { return Err(format!("bad region {region}").into()) };
        keys.extend((x0..=x1).flat_map(|x| (z0..=z1).map(move |z| ChunkKey::new(x, z, 0))));
    }
    if keys.is_empty() {
        return Err("no regions given".into());
    }
    Ok(keys)
}

fn name(registry: &BlockRegistry, id: u32) -> String {
    match registry.get(id) {
        Some(state) if state.properties.is_empty() => state.name.to_string(),
        Some(state) => format!("{}[{}]", state.name, state.properties),
        None => format!("#{id}"),
    }
}

fn ys(chunk: &Chunk) -> std::ops::Range<i32> {
    let dim = chunk.dimension();
    dim.min_y..dim.min_y + dim.height as i32
}

fn column(world: &LevelDbWorld, key: ChunkKey) -> Result<Chunk> {
    Ok(world.load_column(key)?.ok_or(format!("chunk {} {} is not saved", key.x, key.z))?.chunk)
}

/// Runs of equal (block, liquid, biome), top down.
fn dump(world: &LevelDbWorld, x: i32, z: i32) -> Result<()> {
    let chunk = column(world, ChunkKey::new(x >> 4, z >> 4, 0))?;
    let registry = world.registry();
    let cell = |y| (chunk.block(x, y, z), chunk.liquid(x, y, z), chunk.biome(x, y, z));
    let mut top = ys(&chunk).end - 1;
    for y in ys(&chunk).rev() {
        if y == ys(&chunk).start || cell(y - 1) != cell(y) {
            let (block, liquid, biome) = cell(y);
            let liquid = if liquid == registry.air_id() { String::new() } else { format!(" + {}", name(registry, liquid)) };
            println!("{y:>4}..={top:<4} {}{liquid}  biome {biome:?}", name(registry, block));
            top = y - 1;
        }
    }
    Ok(())
}

/// Every chunk saved with biomes in every section, and its floor (y = -64) all bedrock again.
fn check(world: &LevelDbWorld, keys: &[ChunkKey]) -> Result<()> {
    let registry = world.registry();
    let is_bedrock = |id| registry.get(id).is_some_and(|state| state.name == "minecraft:bedrock");
    let (mut missing, mut no_biomes, mut bad_floor) = (0, 0, 0);
    for &key in keys {
        let Some(saved) = world.load_column(key)? else {
            missing += 1;
            continue;
        };
        let chunk = saved.chunk;
        let floor = ys(&chunk).start;
        no_biomes += usize::from(ys(&chunk).step_by(16).any(|y| chunk.biome(0, y, 0).is_none()));
        bad_floor += usize::from((0..256).any(|i| !is_bedrock(chunk.block(i & 15, floor, i >> 4))));
    }
    println!("{} chunks: {missing} not saved, {no_biomes} without biomes for every section, {bad_floor} with a floor not all bedrock", keys.len());
    if missing + no_biomes + bad_floor > 0 {
        return Err("incomplete".into());
    }
    Ok(())
}

fn diff(a: &LevelDbWorld, b: &LevelDbWorld, keys: &[ChunkKey]) -> Result<()> {
    let registry = a.registry();
    let bare = |id| registry.get(id).map_or("?", |state| state.name.trim_start_matches("minecraft:"));
    let (mut chunks, mut blocks, mut biomes) = (0, 0, 0);
    let mut pairs: std::collections::HashMap<(&str, &str), usize> = Default::default();
    for &key in keys {
        let (ca, cb) = (column(a, key)?, column(b, key)?);
        let before = (blocks, biomes);
        for y in ys(&ca) {
            for i in 0..256 {
                let (x, z) = (key.x * 16 + (i & 15), key.z * 16 + (i >> 4));
                biomes += usize::from(ca.biome(x, y, z) != cb.biome(x, y, z));
                let (ba, bb) = ((ca.block(x, y, z), ca.liquid(x, y, z)), (cb.block(x, y, z), cb.liquid(x, y, z)));
                if ba != bb {
                    blocks += 1;
                    *pairs.entry((bare(ba.0), bare(bb.0))).or_default() += 1;
                }
            }
        }
        chunks += usize::from((blocks, biomes) != before);
    }
    println!("{} chunks compared, {chunks} differ: {blocks} block cells, {biomes} biome cells", keys.len());
    let mut pairs: Vec<_> = pairs.into_iter().collect();
    pairs.sort_by(|p, q| q.1.cmp(&p.1));
    for ((from, to), count) in pairs.iter().take(20) {
        println!("{count:>8}  {from} vs {to}");
    }
    Ok(())
}
