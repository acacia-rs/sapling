//! `structure_replay_draws <seed>`: with `SAPLING_ORACLE_STRUCTURE_ORDER`, every mineshaft piece the
//! replayed structure stage post-processes, in the run's order: `chunk x z, piece box (x0 y0 z0 x1 y1 z1),
//! the chunk generator's draws before it, start, piece index and kind`. scripts/dbg_shaft_cmp.py sets it
//! against a scripts/data/dbg_shaft.json log of the same run. `STEPS=1` adds the marked steps inside a piece.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use sapling_worldgen::OverworldGenerator;
use sapling_worldgen::structure::mineshaft_probe::TRACE;

fn main() {
    let seed = std::env::args().nth(1).expect("usage: structure_replay_draws <seed>");
    let mut generator = OverworldGenerator::new(seed.parse().unwrap());
    oracle_support::replay(&mut generator);
    *TRACE.lock().unwrap() = Some(Vec::new());
    assert!(!generator.structure_replay_writes().is_empty(), "SAPLING_ORACLE_STRUCTURE_ORDER unset, or its run placed nothing");
    let trace = TRACE.lock().unwrap().take().unwrap();
    let steps = std::env::var_os("STEPS").is_some();
    for (start, chunk, piece, step, draws) in trace {
        if step == "piece" {
            let (description, b) = generator.mineshaft_pieces(start.0, start.1)[piece].clone();
            println!("{} {} {} {} {} {} {} {} {draws} start {},{} piece {piece} {description}", chunk.0, chunk.1, b.x0, b.y0, b.z0, b.x1, b.y1, b.z1, start.0, start.1);
        } else if steps {
            println!("  {step} {draws}");
        }
    }
}
