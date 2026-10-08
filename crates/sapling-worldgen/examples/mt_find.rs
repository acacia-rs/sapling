//! `mt_find <seed> <cx> <cz> <rule name> <bound>:<value>...`: where in a feature rule's placing
//! generator (seeded as decorate.rs seeds it) a run of consecutive `nextInt(bound) == value`
//! draws occurs, within the first 20000 draws. For finding how many draws BDS spent before a
//! feature it placed.

use sapling_worldgen::random::{LargeFeatureSeed, Mt19937};

fn hashed_string(name: &str) -> u64 {
    name.bytes().fold(0xCBF2_9CE4_8422_2325u64, |hash, byte| hash.wrapping_mul(0x100_0000_01B3) ^ byte as u64)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, cx, cz, rule, wanted @ ..] = &args[..] else { panic!("usage: mt_find <seed> <cx> <cz> <rule> <bound>:<value>...") };
    let seed: i64 = seed.parse().unwrap();
    let chunk_seed = LargeFeatureSeed::new(seed as u32).chunk(cx.parse().unwrap(), cz.parse().unwrap());
    let hash = hashed_string(rule) as u32;
    let rule_seed = chunk_seed ^ hash.wrapping_add(0x9E37_79B9).wrapping_add(chunk_seed << 6).wrapping_add(chunk_seed >> 2);
    // `b:v` wants nextInt(b) == v, `b<v` wants it below v, `b>v` at least v.
    let wanted: Vec<(u32, u32, char)> = wanted
        .iter()
        .map(|w| {
            let at = w.find([':', '<', '>']).expect("b:v, b<v or b>v");
            (w[..at].parse().unwrap(), w[at + 1..].parse().unwrap(), w.as_bytes()[at] as char)
        })
        .collect();
    let mut mt = Mt19937::new(rule_seed);
    let draws: Vec<u32> = (0..60000).map(|_| mt.next_u32()).collect();
    if let Ok(range) = std::env::var("DUMP") {
        let (a, b) = range.split_once("..").unwrap();
        for i in a.parse::<usize>().unwrap()..b.parse().unwrap() {
            let raw = draws[i];
            println!("{i}: raw {raw} %80 {} %14 {} %3 {} %2 {} float {}", raw % 80, raw % 14, raw % 3, raw % 2, sapling_worldgen::random::unit(raw));
        }
        return;
    }
    for i in 0..draws.len() - wanted.len() {
        let ok = |k: usize, (bound, value, op): (u32, u32, char)| match op {
            '<' => draws[i + k] % bound < value,
            '>' => draws[i + k] % bound >= value,
            _ => draws[i + k] % bound == value,
        };
        if wanted.iter().enumerate().all(|(k, &w)| ok(k, w)) {
            println!("match at draw {i}");
        }
    }
}
