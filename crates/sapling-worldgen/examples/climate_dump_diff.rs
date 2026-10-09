//! `climate_dump_diff <seed> <x0> <z0> <width> <dump.bin>`: BDS's `ChunkLocalNoiseCache` entries (0x28
//! bytes per quart column, x outer; the struct `_computeDensityCells` 14822d4b0 takes in r9 holds
//! x0, z0, width and at +0x10 the entries) against our `QuartClimate`: per field, how many columns
//! differ and by how many f32 steps at most.

use sapling_worldgen::OverworldGenerator;

const ENTRY: usize = 0x28;
const FIELDS: [&str; 10] = ["shifted x", "shifted z", "continentalness", "weirdness", "erosion", "offset", "factor", "jagged", "temperature", "humidity"];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, x0, z0, width, path] = &args[..] else { panic!("usage: climate_dump_diff <seed> <x0> <z0> <width> <dump.bin>") };
    let (x0, z0, width): (i32, i32, i32) = (x0.parse().unwrap(), z0.parse().unwrap(), width.parse().unwrap());
    let bytes = std::fs::read(path).expect("dump file");
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let climate = &generator.density().climate;
    let mut worst = [(0u32, 0i64); 10];
    for dx in 0..width {
        for dz in 0..width {
            let at = (dx * width + dz) as usize * ENTRY;
            let bds: Vec<f32> = (0..10).map(|k| f32::from_le_bytes(bytes[at + k * 4..at + k * 4 + 4].try_into().unwrap())).collect();
            let c = climate.sample(x0 + dx, z0 + dz);
            let ours = [c.shifted_x, c.shifted_z, c.continentalness, c.weirdness, c.erosion, c.offset, c.factor, c.jagged, c.temperature, c.humidity];
            if dx == 0 && dz < 2 {
                println!("column {} {}: ours {ours:?}\n               bds {bds:?}", x0 + dx, z0 + dz);
            }
            for k in 0..10 {
                let steps = (ours[k].to_bits() as i32 as i64 - bds[k].to_bits() as i32 as i64).abs();
                if steps != 0 {
                    worst[k] = (worst[k].0 + 1, worst[k].1.max(steps));
                }
            }
        }
    }
    for (name, (count, steps)) in FIELDS.iter().zip(worst) {
        println!("{name}: {count} of {} columns differ, at most {steps} f32 steps", width * width);
    }
}
