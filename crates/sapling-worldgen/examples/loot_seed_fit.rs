//! `loot_seed_fit <level seed> <file>`: for each `x y z seed` line (a BDS `LootTableSeed`), where it
//! occurs in candidate generators: the chunk's structure MT (a u32 or `nextInt()` among the first
//! 4000), the chunk's decoration Xoroshiro (low 32 bits of `nextLong`), and
//! Java randoms seeded from the cell (`Mth.getSeed`, narrow and wide x), as the low 32 bits of
//! `nextLong` after k draws.

use std::io::BufRead;

use sapling_worldgen::random::{JavaRandom, LargeFeatureSeed, Mt19937, Xoroshiro, position_seed, wide_position_seed};

fn main() {
    let level: i64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let seeds = LargeFeatureSeed::new(level as u32);
    let file = std::fs::File::open(std::env::args().nth(2).unwrap()).unwrap();
    for line in std::io::BufReader::new(file).lines() {
        let line = line.unwrap();
        let v: Vec<i64> = line.split_whitespace().map(|w| w.parse().unwrap()).collect();
        let [x, y, z, want] = v[..] else { continue };
        let (x, y, z, want) = (x as i32, y as i32, z as i32, want as i32 as u32);
        let mut found = Vec::new();
        for (dx, dz) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (cx, cz) = (x.div_euclid(16) + dx, z.div_euclid(16) + dz);
            let mut mt = Mt19937::new(seeds.chunk(cx, cz));
            for i in 0..4000 {
                let u = mt.next_u32();
                if u == want {
                    found.push(format!("mt({cx},{cz})#{i}"));
                } else if u >> 1 == want {
                    found.push(format!("mt({cx},{cz})#{i}>>1"));
                }
            }
        }
        let (cx, cz) = (x.div_euclid(16), z.div_euclid(16));
        let mut xo = Xoroshiro::from_seed(level);
        let (a, b) = (xo.next_long() | 1, xo.next_long() | 1);
        let mut xo = Xoroshiro::from_seed(((cx as i64 as u64).wrapping_mul(a).wrapping_add((cz as i64 as u64).wrapping_mul(b)) ^ level as u64) as i64);
        for i in 0..4000 {
            if xo.next_long() as u32 == want {
                found.push(format!("xo#{i}"));
            }
        }
        for (name, s) in [("narrow", position_seed(x, y, z)), ("wide", wide_position_seed(x, y, z))] {
            for k in 0..8 {
                let mut r = JavaRandom::new(s);
                for _ in 0..k {
                    r.next_float();
                }
                if r.next_long() as u32 == want {
                    found.push(format!("{name}+{k}"));
                }
            }
        }
        println!("{x} {y} {z} {}: {}", want as i32, if found.is_empty() { "-".into() } else { found.join(" ") });
    }
}
