//! `stages <seed> <block x> <block z> <y0> <y1>`: a column's blocks from y1 down to y0 after each
//! stage (carvers, structures, lakes, features), the structure features near its chunk, and each
//! of the 3x3 source chunks' feature writes into it (each decorated alone, in source order).

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().expect("integer argument")).collect();
    let [seed, x, z, y0, y1] = args[..] else { panic!("usage: stages <seed> <x> <z> <y0> <y1>") };
    let (x, z, y0, y1) = (x as i32, z as i32, y0 as i32, y1 as i32);
    let (cx, cz) = (x >> 4, z >> 4);
    let generator = OverworldGenerator::new(seed);
    let mut carved = generator.surfaced_blocks(cx, cz);
    generator.carve(&mut carved, cx, cz);
    let mut structured = carved.clone();
    generator.place_structures(&mut structured, cx, cz);
    let mut laked = structured.clone();
    generator.decorate(&mut laked, cx, cz);
    let mut featured = laked.clone();
    generator.place_features(&mut featured, cx, cz);
    let (lx, lz) = ((x & 15) as usize, (z & 15) as usize);
    println!("   y  carved / structures / lakes / features");
    for y in (y0..=y1).rev() {
        let stages = [&carved, &structured, &laked, &featured].map(|s| format!("{:?}", s.block(lx, y, lz)));
        println!("{y:>4}  {}", stages.join(" / "));
    }
    for (name, at) in generator.structure_features_near(cx, cz) {
        println!("structure feature {name} at {at:?}");
    }
    for (sx, sz) in (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| (cx + dx, cz + dz))) {
        for ([wx, y, wz], block, replaced) in generator.source_feature_writes(sx, sz) {
            if (wx, wz) == (x, z) && (y0..=y1).contains(&y) {
                println!("source {sx},{sz} writes {block:?} over {replaced:?} at y {y}");
            }
        }
    }
}
