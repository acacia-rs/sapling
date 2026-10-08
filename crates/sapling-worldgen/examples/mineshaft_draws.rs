//! `mineshaft_draws <seed> <world> <cx> <cz> [reach]`: for each mineshaft piece a chunk
//! post-processes, in order, our chunk generator's draw count before it and the shift k (within
//! `reach`, default 40) that makes its mineshaft-block cells in the chunk match BDS best, with
//! the score (matches - mismatches over cells either side has such a block, BDS's two runs
//! agreeing) at k = 0 and at the best k. A piece whose best k leaves 0 is where draws desync.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use oracle_support::{bds_cell, open};
use sapling_leveldb::ChunkKey;
use sapling_worldgen::OverworldGenerator;
use sapling_worldgen::structure::mineshaft_probe::{SHIFT, Shift, TRACE};

const PALETTE: [&str; 8] = ["planks", "fence", "rail", "torch", "web", "chain", "_log", "mob_spawner"];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, world, cx, cz, rest @ ..] = &args[..] else { panic!("usage: mineshaft_draws <seed> <world> cx cz [reach]") };
    let chunk: (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let reach: i64 = rest.first().map_or(40, |r| r.parse().unwrap());
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let (primary, twin) = open(world).expect("oracle world");
    let key = ChunkKey::new(chunk.0, chunk.1, 0);
    let (bds, other) = (primary.load_column(key).unwrap().expect("saved").chunk, twin.load_column(key).unwrap().expect("saved").chunk);
    *TRACE.lock().unwrap() = Some(Vec::new());
    generator.pieced_uncached(chunk.0, chunk.1);
    let trace = TRACE.lock().unwrap().take().unwrap();
    let palette = |name: &str| PALETTE.iter().any(|p| name.contains(p));
    if std::env::var_os("TRACE").is_some() {
        for (start, _, piece, step, draws) in trace.iter().filter(|t| t.1 == chunk) {
            println!("trace {start:?} piece {piece} {step} {draws}");
        }
        return;
    }
    for (start, at, piece, _, draws) in trace.into_iter().filter(|t| t.1 == chunk && t.3 == "piece") {
        let (description, bb) = generator.mineshaft_pieces(start.0, start.1)[piece].clone();
        let score = |shift: Option<u32>| {
            *SHIFT.lock().unwrap() = shift.map(|draws| Shift { start, chunk: at, piece, draws });
            let ours = generator.pieced_uncached(chunk.0, chunk.1);
            *SHIFT.lock().unwrap() = None;
            let mut score = 0i32;
            for x in (bb.x0 - 1).max(chunk.0 * 16)..=(bb.x1 + 1).min(chunk.0 * 16 + 15) {
                for z in (bb.z0 - 1).max(chunk.1 * 16)..=(bb.z1 + 1).min(chunk.1 * 16 + 15) {
                    for y in bb.y0 - 1..=bb.y1 {
                        let block = ours.block((x & 15) as usize, y, (z & 15) as usize);
                        let cell = bds_cell(&primary, &bds, x, y, z);
                        if (!palette(block.name()) && !palette(&cell.block)) || cell != bds_cell(&twin, &other, x, y, z) {
                            continue;
                        }
                        score += if block.name() == cell.block { 1 } else { -1 };
                    }
                }
            }
            score
        };
        let base = score(None);
        if std::env::var_os("ALL").is_some() {
            let all: Vec<String> = (-reach..=reach).filter(|k| draws as i64 + k >= 0).map(|k| format!("{k}:{}", score(Some((draws as i64 + k) as u32)))).collect();
            println!("piece {piece}: {}", all.join(" "));
        }
        let best = (-reach..=reach)
            .filter(|k| draws as i64 + k >= 0)
            .map(|k| (score(Some((draws as i64 + k) as u32)), -k.abs(), k))
            .max()
            .unwrap();
        println!("{start:?} piece {piece:>3} draws {draws:>5}: k=0 {base:>4}, best k={:>4} {:>4}  {description} {bb:?}", best.2, best.0);
    }
}
