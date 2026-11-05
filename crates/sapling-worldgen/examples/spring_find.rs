//! `spring_find <seed> <cx0> <cz0> <cx1> <cz1> [stride]`: every sulfur spring the chunks' decoration traces show
//! (a trace reaches about 3 chunks round its own), with its template, rotation and the block under it. For
//! choosing spring canon regions; a stride of 4 still sees most springs, each several times.
//! Only chunks with sulfur caves (biome 194) under their middle or corners, y -40 to 40, are traced.

use sapling_worldgen::OverworldGenerator;

const SULFUR_CAVES: u32 = 194;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|arg| arg.parse().expect("a number")).collect();
    let (&[seed, x0, z0, x1, z1], stride) = (&args[..5].try_into().expect("usage: spring_find <seed> cx0 cz0 cx1 cz1 [stride]"), args.get(5).map_or(1, |&s| s as usize));
    let generator = OverworldGenerator::new(seed);
    let climate = &generator.density().climate;
    let points = [(8, 8), (0, 0), (15, 0), (0, 15), (15, 15)];
    let has_caves = |cx: i32, cz: i32| points.iter().any(|&(dx, dz)| (-40..=40).step_by(8).any(|y| climate.biome_at(cx * 16 + dx, y, cz * 16 + dz) == SULFUR_CAVES));
    for cx in (x0 as i32..=x1 as i32).step_by(stride) {
        for cz in (z0 as i32..=z1 as i32).step_by(stride).filter(|&cz| has_caves(cx, cz)) {
            for line in generator.source_feature_trace(cx, cz).iter().filter(|line| line.contains("sulfur spring")) {
                println!("{cx} {cz}{line}");
            }
        }
    }
}
