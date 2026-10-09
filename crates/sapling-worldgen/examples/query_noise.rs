//! `query_noise <cx0> <cz0> <cx1> <cz1> [scale] [divisor]`: `math.ceil(query.noise(originx / divisor,
//! originz / divisor) * scale)` per chunk (rows z, columns x; scale 80, divisor 80: the lukewarm
//! kelp rule's iterations), to set beside the kelp columns BDS grew.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let n: Vec<f32> = std::env::args().skip(1).map(|a| a.parse().expect("number")).collect();
    let (scale, divisor) = (n.get(4).copied().unwrap_or(80.0), n.get(5).copied().unwrap_or(80.0));
    for cz in n[1] as i32..=n[3] as i32 {
        let row: Vec<String> = (n[0] as i32..=n[2] as i32)
            .map(|cx| format!("{:>4}", (OverworldGenerator::query_noise((cx * 16) as f32 / divisor, (cz * 16) as f32 / divisor) * scale).ceil()))
            .collect();
        println!("z {cz:>4} {}", row.join(""));
    }
}
