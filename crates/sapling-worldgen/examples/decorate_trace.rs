//! `decorate_trace <seed> <x0> <z0> <x1> <z1>...`: decorates every source chunk of the regions (and their ring),
//! so SAPLING_TRACE_RULES prints the rules' feature positions to stderr.
use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(args[0]);
    for r in args[1..].chunks(4) {
        let (x0, z0, x1, z1) = (r[0] as i32, r[1] as i32, r[2] as i32, r[3] as i32);
        for cx in x0 - 1..=x1 + 1 {
            for cz in z0 - 1..=z1 + 1 {
                let _ = generator.source_feature_writes(cx, cz);
            }
        }
    }
}
