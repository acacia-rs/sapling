//! `nether_cells <seed> <world dir> <chunk x> <chunk z>`: every cell of one finished Nether chunk that differs from
//! the BDS-saved world, under the decoration order in `order.txt` beside the world (as tests/nether_oracle.rs).
use std::path::Path;

use sapling_leveldb::{ChunkKey, LevelDbWorld};
use sapling_worldgen::NetherGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, cx, cz] = &args[..] else { panic!("usage: nether_cells <seed> <world> <cx> <cz>") };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let path = Path::new(world);
    let mut generator = NetherGenerator::new(seed.parse().unwrap());
    if let Ok(text) = std::fs::read_to_string(path.parent().unwrap().join("order.txt")) {
        let rows: Vec<Vec<i64>> = text.lines().map(|line| line.split(' ').map(|v| v.parse().unwrap()).collect()).collect();
        let order: Vec<(i32, i32)> = rows.iter().map(|row| (row[0] as i32, row[1] as i32)).collect();
        let spent: Vec<Option<u32>> = rows.iter().map(|row| row.get(2).map(|&draws| draws as u32)).collect();
        generator.set_decoration_order(&order, Some((0, &spent)));
    }
    let world = LevelDbWorld::open_in_memory(path).unwrap();
    let registry = world.registry();
    let chunk = world.load_column(ChunkKey::new(cx, cz, 1)).unwrap().expect("chunk saved").chunk;
    let ours = generator.chunk_blocks(cx, cz);
    for y in 0..128 {
        for z in 0..16 {
            for x in 0..16 {
                let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                let block = ours.block(x, y, z);
                let saved = registry.get(chunk.block(wx, y, wz));
                let (name, states) = saved.map_or(("?".to_string(), String::new()), |s| (s.name.trim_start_matches("minecraft:").to_string(), s.properties.to_string()));
                if block.name() != name || block.states() != states {
                    println!("{wx} {y} {wz}: {}[{}] / {name}[{states}]", block.name(), block.states());
                }
            }
        }
    }
}
