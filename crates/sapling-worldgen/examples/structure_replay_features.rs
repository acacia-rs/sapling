//! `structure_replay_features <seed> [substring [chunk]]` (a third argument like `"in (-71, 75)"` also prints
//! that chunk's city pieces with the chunk generator's draws before each): with `SAPLING_ORACLE_STRUCTURE_ORDER`, every feature a
//! piece placed in the replayed structure stage, in the run's order (`element <feature> at <origin> from
//! <chunk>`), and under the elements whose line holds the substring their writes in the order made. To set
//! beside a BDS log of the feature's entries and writes during structure post-processes.
#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, rest @ ..] = &args[..] else { panic!("usage: structure_replay_features <seed> [substring]") };
    let mut generator = OverworldGenerator::new(seed.parse().unwrap());
    oracle_support::replay(&mut generator);
    let mut listed = false;
    for line in generator.structure_replay_trace() {
        if line.starts_with("element ") {
            listed = rest.first().is_some_and(|want| line.contains(want.as_str()));
            println!("{line}");
        } else if line.starts_with("piece ") {
            if rest.get(1).is_some_and(|chunk| line.contains(chunk.as_str())) {
                println!("{line}");
            }
        } else if listed {
            println!("{line}");
        }
    }
}
