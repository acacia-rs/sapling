//! `prelim <seed> <block x> <block z>`: the preliminary surface level of the block's quart column
//! and its neighbours, as `getPreliminarySurfaceLevel` (14822e160) returns it.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().expect("integer argument")).collect();
    let [seed, x, z] = args[..] else { panic!("usage: prelim <seed> <x> <z>") };
    let generator = OverworldGenerator::new(seed);
    let (qx, qz) = ((x as i32) >> 2, (z as i32) >> 2);
    for dz in -1..=1 {
        let row: Vec<String> = (-1..=1).map(|dx| format!("{:>5}", generator.density().preliminary_surface(qx + dx, qz + dz))).collect();
        println!("{}", row.join(" "));
    }
}
