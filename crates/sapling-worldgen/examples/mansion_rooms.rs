//! `mansion_rooms <seed> <radius> <template...>`: every mansion starting within `radius` chunks of
//! chunk (0, 0) that holds one of the named templates (e.g. `1x2_a4`), with those pieces' boxes and
//! the chunks the whole mansion covers, nearest first.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, radius, wanted @ ..] = &args[..] else { panic!("usage: mansion_rooms <seed> <radius> <template...>") };
    let generator = OverworldGenerator::new(seed.parse().expect("seed"));
    let mut found = generator.mansion_rooms(0, 0, radius.parse().expect("radius"));
    found.sort_by_key(|((x, z), _)| x * x + z * z);
    println!("{} mansions; the nearest's rooms: {:?}", found.len(), found.first().map(|(_, pieces)| pieces.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>()));
    for (start, pieces) in found {
        let hits: Vec<_> = pieces.iter().filter(|(name, _)| wanted.iter().any(|w| name == w || name.ends_with(&format!("/{w}")))).collect();
        if hits.is_empty() {
            continue;
        }
        let bb = pieces.iter().skip(1).fold(pieces[0].1, |b, p| b.union(&p.1));
        println!("start {start:?} chunks {},{},{},{}", bb.x0 >> 4, bb.z0 >> 4, bb.x1 >> 4, bb.z1 >> 4);
        for (name, bb) in hits {
            println!("  {name} {bb:?}");
        }
    }
}
