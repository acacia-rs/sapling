//! `room_probe <seed> <x> <z> <reach>`: cave segments centred within `reach` of a column, with and
//! without the carvers' skip gate, as block y (0-based y - 64).

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().expect("integer argument")).collect();
    let [seed, x, z, reach] = args[..] else { panic!("usage: room_probe <seed> <x> <z> <reach>") };
    let generator = OverworldGenerator::new(seed);
    for gate in [true, false] {
        println!("skip gate {gate}:");
        for (carver, c, h, v) in generator.cave_segments_near(x as i32, z as i32, reach as f32, gate) {
            println!("  carver {carver} at {:.1} {:.1} {:.1} r {h:.2}/{v:.2}", c[0], c[1] - 64.0, c[2]);
        }
    }
}
