//! `end_gateways <seed> <cx0> <cz0> <cx1> <cz1>`: the end gateways the End's decoration places in those chunks.

use sapling_worldgen::{Block, EndGenerator};

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|v| v.parse().unwrap()).collect();
    let [seed, x0, z0, x1, z1] = args[..] else { panic!("usage: end_gateways <seed> cx0 cz0 cx1 cz1") };
    let generator = EndGenerator::new(seed);
    for cx in x0 as i32..=x1 as i32 {
        for cz in z0 as i32..=z1 as i32 {
            if generator.island_height(cx, cz, 1, 1) <= 40.0 {
                continue;
            }
            let shape = generator.chunk_blocks(cx, cz);
            for y in 0..256 {
                for i in 0..256 {
                    if shape.block(i & 15, y, i >> 4) == Block::EndGateway {
                        println!("gateway {} {y} {}", cx * 16 + (i & 15) as i32, cz * 16 + (i >> 4) as i32);
                    }
                }
            }
        }
    }
}
