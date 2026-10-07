//! `start_probe <seed> <chunk x> <chunk z> [radius]`: what a structure start check sees at a
//! chunk: the preliminary surface at its corner quart and the point biomes within `radius` of
//! its centre on the 4-block lattice, at that surface.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let [seed, cx, cz, rest @ ..] = &args[..] else { panic!("usage: start_probe <seed> <chunk x> <chunk z> [radius]") };
    let (cx, cz) = (*cx as i32, *cz as i32);
    let radius = rest.first().copied().unwrap_or(2) as i32;
    let generator = OverworldGenerator::new(*seed);
    let density = generator.density();
    let surface = density.preliminary_surface(cx * 4, cz * 4);
    println!("chunk {cx},{cz}: preliminary surface {surface}");
    let (x, z) = (cx * 16 + 8, cz * 16 + 8);
    for bx in (x - radius..=x + radius).step_by(4) {
        for bz in (z - radius..=z + radius).step_by(4) {
            let chunk = generator.chunk_biomes(bx >> 4, bz >> 4);
            let span = std::env::var("PROBE_SPAN").ok().and_then(|s| s.parse().ok()).unwrap_or(radius);
            let ids: Vec<String> = (surface - span..=surface + span)
                .step_by(4)
                .map(|by| format!("y{by}:{}/{}", density.climate.biome_at(bx, by, bz), chunk.biome((bx & 15) as usize, by, (bz & 15) as usize)))
                .collect();
            println!("  {bx},{bz}: {}", ids.join(" "));
        }
    }
}
