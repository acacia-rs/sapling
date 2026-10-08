//! `column_heights <seed> <cx> <cz>`: one above the highest block that is neither air nor a
//! plant in each column of a chunk after the structure stage, rows z, columns x.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, cx, cz] = &args[..] else { panic!("usage: column_heights <seed> <cx> <cz>") };
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let shape = generator.loaded_blocks(cx.parse().unwrap(), cz.parse().unwrap());
    for z in 0..16 {
        let row: Vec<String> = (0..16)
            .map(|x| {
                let top = (-64..320).rev().find(|&y| {
                    let block = shape.block(x, y, z);
                    block.name() != "air" && !block.is_plant()
                });
                format!("{:>4}", top.map_or(-64, |y| y + 1))
            })
            .collect();
        println!("{}", row.join(""));
    }
}
