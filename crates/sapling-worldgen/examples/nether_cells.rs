//! `nether_cells <seed> <world dir> <chunk x> <chunk z> [x,y,z]`: every cell of one finished Nether chunk that differs
//! from the BDS-saved world, under the decoration and structure orders beside the world (as tests/nether_oracle.rs);
//! with a cell, its 3x3x3 neighbourhood instead, ours / BDS.
use std::path::Path;

use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_worldgen::{NetherGenerator, StructureStep};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (seed, world, cx, cz) = match &args[..] {
        [seed, world, cx, cz, ..] => (seed, world, cx, cz),
        _ => panic!("usage: nether_cells <seed> <world> <cx> <cz> [x,y,z]"),
    };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let path = Path::new(world);
    let mut generator = NetherGenerator::new(seed.parse().unwrap());
    let dir = path.parent().unwrap();
    if let Ok(text) = std::fs::read_to_string(dir.join("order.txt")) {
        let rows: Vec<Vec<i64>> = text.lines().map(|line| line.split(' ').map(|v| v.parse().unwrap()).collect()).collect();
        let order: Vec<(i32, i32)> = rows.iter().map(|row| (row[0] as i32, row[1] as i32)).collect();
        let spent: Vec<Option<u32>> = rows.iter().map(|row| row.get(2).map(|&draws| draws as u32)).collect();
        generator.set_decoration_order(&order, Some((0, &spent)));
    }
    if let Ok(text) = std::fs::read_to_string(dir.join("structure-order.txt")) {
        let steps: Vec<StructureStep> = text.lines().filter_map(step).collect();
        generator.set_structure_order(&steps);
    }
    let world = LevelDbWorld::open_in_memory(path).unwrap();
    let registry = world.registry();
    let saved = |x: i32, y: i32, z: i32| {
        let chunk = world.load_column(ChunkKey::new(x >> 4, z >> 4, 1)).unwrap().expect("chunk saved").chunk;
        registry.get(chunk.block(x, y, z)).map_or("?".to_string(), |s| format!("{}[{}]", s.name.trim_start_matches("minecraft:"), s.properties))
    };
    let ours = |x: i32, y: i32, z: i32| {
        let shape = generator.chunk_blocks(x >> 4, z >> 4);
        let block = shape.block((x & 15) as usize, y, (z & 15) as usize);
        format!("{}[{}]", block.name(), block.states())
    };
    if args.get(4).is_some_and(|a| a == "heights") {
        let column = world.load_column(ChunkKey::new(cx, cz, 1)).unwrap().expect("chunk saved");
        let heights = column.data3d.expect("Data3D").heightmap;
        heights.chunks(16).for_each(|row| println!("{row:?}"));
        return;
    }
    if let Some(cell) = args.get(4) {
        let [x, y, z]: [i32; 3] = cell.split(',').map(|v| v.parse().unwrap()).collect::<Vec<_>>().try_into().unwrap();
        // A wider radius prints only the cells that are neither air nor netherrack on either side.
        let radius: i32 = args.get(5).map_or(1, |r| r.parse().unwrap());
        for dy in (-radius..=radius).rev() {
            for dz in -radius..=radius {
                for dx in -radius..=radius {
                    let (px, py, pz) = (x + dx, y + dy, z + dz);
                    let (a, b) = (ours(px, py, pz), saved(px, py, pz));
                    if radius == 1 || ![&a, &b].iter().all(|s| s.as_str() == "air[]" || s.as_str() == "netherrack[]") {
                        println!("{px} {py} {pz}: {a} / {b}");
                    }
                }
            }
        }
        return;
    }
    let shape = generator.chunk_blocks(cx, cz);
    for y in 0..128 {
        for z in 0..16 {
            for x in 0..16 {
                let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                let block = shape.block(x, y, z);
                let (theirs, mine) = (saved(wx, y, wz), format!("{}[{}]", block.name(), block.states()));
                if theirs != mine {
                    println!("{wx} {y} {wz}: {mine} / {theirs}");
                }
            }
        }
    }
}

/// A structure-order.txt line: `load x z`, or `x z` for a post-process.
fn step(line: &str) -> Option<StructureStep> {
    let chunk = |text: &str| {
        let mut words = text.split(' ').map(|v| v.parse().ok());
        Some((words.next()??, words.next()??))
    };
    match line.strip_prefix("load ") {
        Some(rest) => Some(StructureStep::Load(chunk(rest)?)),
        None => Some(StructureStep::Post(chunk(line)?)),
    }
}
