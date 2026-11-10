//! Diffs the columns sapling serves over a world folder, its missing columns generated
//! (`Generated`), against the same columns of a world BDS saved after generating them over that folder:
//! `folder_generation_diff <seed> <folder before, or -> <BDS world after> <cx0> <cz0> <cx1> <cz1>`
//! (`-`: no folder, every column generated). Prints per chunk (rows z) the differing cells, `-` where
//! BDS saved none, the folder's and BDS's `FinalizedState` of each, then the commonest differing pairs.
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use sapling_leveldb::{ChunkKey, LevelDbWorld, Tag};
use sapling_world::{BlockRegistry, ChunkPos, Generated, LevelDbStorage, MemoryStorage, WorldStorage};

fn finalized(world: Option<&LevelDbWorld>, x: i32, z: i32) -> String {
    let Some(world) = world else { return "-".into() };
    let key = ChunkKey::new(x, z, 0);
    match world.db().get(&key.tag(Tag::FinalizedState)).ok().flatten() {
        Some(bytes) if bytes.len() >= 4 => u32::from_le_bytes(bytes[..4].try_into().unwrap()).to_string(),
        _ if world.db().get(&key.tag(Tag::Version)).ok().flatten().is_some() => "?".into(),
        _ => ".".into(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, before, after, rest @ ..] = &args[..] else { return Err("usage: folder_generation_diff <seed> <before|-> <after> <cx0> <cz0> <cx1> <cz1>".into()) };
    let seed: i64 = seed.parse()?;
    let corners: Vec<i32> = rest.iter().map(|arg| arg.parse()).collect::<Result<_, _>>()?;
    let [x0, z0, x1, z1] = corners[..] else { return Err("four chunk corners".into()) };
    let folder = (before != "-").then(|| LevelDbWorld::open_in_memory(Path::new(before))).transpose()?;
    let storage: Arc<dyn WorldStorage> = match before.as_str() {
        "-" => Arc::new(Generated::with_areas(MemoryStorage::default(), seed, 1)),
        path => Arc::new(Generated::with_areas(LevelDbStorage::open_in_memory(Path::new(path))?, seed, 1)),
    };
    let bds = LevelDbWorld::open_in_memory(Path::new(after))?;
    let registry = BlockRegistry::vanilla();
    let name = |id: u32| registry.get(id).map_or_else(|| format!("#{id}"), |state| format!("{}{:?}", state.name.trim_start_matches("minecraft:"), state.properties));
    let mut pairs: HashMap<(u32, u32, u32, u32), usize> = HashMap::new();
    let mut total = 0;
    println!("per chunk: differing cells (folder state/BDS state)");
    for z in z0..=z1 {
        let mut row = format!("z {z:>5}:");
        for x in x0..=x1 {
            let states = format!("({}/{})", finalized(folder.as_ref(), x, z), finalized(Some(&bds), x, z));
            let Some(theirs) = bds.load_column(ChunkKey::new(x, z, 0))? else {
                row += &format!(" {:>6}{states:<6}", "-");
                continue;
            };
            let ours = storage.load_column(ChunkPos::new(x, z), 0)?.ok_or("no column")?;
            let mut differing = 0;
            for y in -64..320 {
                for (bx, bz) in (0..16).flat_map(|bx| (0..16).map(move |bz| (bx, bz))) {
                    let a = (ours.chunk.block(bx, y, bz), ours.chunk.liquid(bx, y, bz));
                    let b = (theirs.chunk.block(bx, y, bz), theirs.chunk.liquid(bx, y, bz));
                    if a != b {
                        differing += 1;
                        *pairs.entry((a.0, a.1, b.0, b.1)).or_default() += 1;
                    }
                }
            }
            total += differing;
            row += &format!(" {differing:>6}{states:<6}");
        }
        println!("{row}");
    }
    println!("total {total}");
    let mut pairs: Vec<_> = pairs.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    for ((ours, ours_liquid, theirs, theirs_liquid), count) in pairs.into_iter().take(25) {
        println!("{count:>7} ours {} +{} / bds {} +{}", name(ours), name(ours_liquid), name(theirs), name(theirs_liquid));
    }
    Ok(())
}
