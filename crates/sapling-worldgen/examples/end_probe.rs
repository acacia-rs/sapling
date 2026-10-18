//! `end_probe <seed> <cx0> <cz0> <cx1> <cz1>`: per chunk, the island height the End's decoration gates on and its
//! post-process generator's first draws (feature/end.rs reads them as: island chance, x, y, z).

use sapling_worldgen::EndGenerator;
use sapling_worldgen::random::{LargeFeatureSeed, Mt19937};

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|v| v.parse().unwrap()).collect();
    let [seed, x0, z0, x1, z1] = args[..] else { panic!("usage: end_probe <seed> cx0 cz0 cx1 cz1") };
    let generator = EndGenerator::new(seed);
    let large = LargeFeatureSeed::new(seed as u32);
    for cx in x0 as i32..=x1 as i32 {
        for cz in z0 as i32..=z1 as i32 {
            let mut random = Mt19937::new(large.chunk(cx, cz));
            let draws: Vec<u32> = (0..6).map(|_| random.next_u32()).collect();
            let height = generator.island_height(cx, cz, 1, 1);
            let shown: Vec<String> = draws.iter().map(|d| format!("{}/{}", d % 14, d & 15)).collect();
            println!("{cx},{cz} height {height} draws %14/&15: {}", shown.join(" "));
        }
    }
}
