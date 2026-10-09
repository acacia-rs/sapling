//! `octave_offsets <seed> [<main.bin> <min.bin> <max.bin>]`: the blended noise's octave offsets, and
//! against BDS's octave objects (0x818 bytes each, offsets first; `OverworldGeneratorMultinoise`
//! +0x2d0, +0x280, +0x2a8, dumped with scripts/bds_dbg.py's `dump_ptr`) the ones that differ.

use sapling_worldgen::OverworldGenerator;

const OCTAVE_BYTES: usize = 0x818;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, files @ ..] = &args[..] else { panic!("usage: octave_offsets <seed> [main min max]") };
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let ours = generator.density().blended_offsets();
    for (part, (name, octaves)) in ["main", "min", "max"].iter().zip(&ours).enumerate() {
        let bds = files.get(part).map(|path| std::fs::read(path).expect("octave dump"));
        for (i, offsets) in octaves.iter().enumerate() {
            let Some(bds) = &bds else {
                println!("{name} {i}: {offsets:?}");
                continue;
            };
            let at = i * OCTAVE_BYTES;
            let theirs: Vec<f32> = (0..3).map(|k| f32::from_le_bytes(bds[at + k * 4..at + k * 4 + 4].try_into().unwrap())).collect();
            let steps: Vec<i64> = (0..3).map(|k| offsets[k].to_bits() as i64 - theirs[k].to_bits() as i64).collect();
            let flags = &bds[at + 0x80c..at + 0x818];
            println!("{name} {i}: ours {offsets:?} bds {theirs:?} steps {steps:?} flags {flags:?}");
        }
    }
}
