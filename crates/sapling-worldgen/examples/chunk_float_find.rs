//! `chunk_float_find <seed> <cx> <cz> <chance> <bits>`: where in a chunk's structure generator
//! (the large-feature chunk seed) a run of floats lies whose `< chance` outcomes spell `bits`
//! (`1` below, `0` not, `?` either), within the first 20000 draws.

use sapling_worldgen::random::{LargeFeatureSeed, Mt19937};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, cx, cz, chance, bits] = &args[..] else { panic!("usage: chunk_float_find <seed> <cx> <cz> <chance> <bits>") };
    let seed: i64 = seed.parse().unwrap();
    let chance: f32 = chance.parse().unwrap();
    let mut mt = Mt19937::new(LargeFeatureSeed::new(seed as u32).chunk(cx.parse().unwrap(), cz.parse().unwrap()));
    let floats: Vec<f32> = (0..20000).map(|_| mt.next_float()).collect();
    if let Some((a, b)) = bits.strip_prefix("dump:").and_then(|r| r.split_once("..")) {
        for (i, f) in floats.iter().enumerate().take(b.parse().unwrap()).skip(a.parse().unwrap()) {
            println!("{i}: {f} {}", u8::from(*f < chance));
        }
        return;
    }
    let bits: Vec<u8> = bits.bytes().collect();
    for i in 0..floats.len() - bits.len() {
        if bits.iter().enumerate().all(|(k, &b)| b == b'?' || (floats[i + k] < chance) == (b == b'1')) {
            println!("match at draw {i}");
        }
    }
}
