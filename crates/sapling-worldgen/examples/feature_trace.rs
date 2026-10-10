//! `feature_trace <seed> <chunk x> <chunk z> [substring]`: the decoration trace of one source chunk
//! (each rule try with its position, result and draw range in the rule's placing generator, each
//! tree attempt under it), keeping the lines that contain the substring. Beside
//! `tree_draws` (BDS's trunks by draw index) it names the first try that desyncs.
//! `SAPLING_ORACLE_ORDER` and `SAPLING_ORACLE_THREAD_SEED` as in tests/oracle.rs.

#![allow(dead_code)]

#[path = "../tests/oracle_support/mod.rs"]
mod oracle_support;

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, cx, cz, rest @ ..] = &args[..] else { panic!("usage: feature_trace <seed> <cx> <cz> [substring]") };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let mut generator = OverworldGenerator::new(seed.parse().unwrap());
    // As the oracle test: a BDS run's decoration order and its one worker's thread generator seed.
    if let Ok(path) = std::env::var("SAPLING_ORACLE_ORDER") {
        let text = std::fs::read_to_string(path).expect("order file");
        let order: Vec<(i32, i32)> = text.lines().filter_map(|line| line.split_once(' ')).map(|(x, z)| (x.parse().unwrap(), z.parse().unwrap())).collect();
        generator.set_decoration_order(&order, oracle_support::thread_seed());
    }
    // The trace holds the earlier phases' sources too.
    let own = format!("({cx}, {cz}) ");
    let mut trees = Vec::new();
    for line in generator.source_feature_trace(cx, cz) {
        // A try's tree lines come before its own line.
        if line.starts_with(' ') {
            trees.push(line);
            continue;
        }
        if line.starts_with("thread after") {
            println!("{line}");
        }
        if line.starts_with(&own) && rest.first().is_none_or(|want| line.contains(want.as_str())) {
            println!("{line}");
            trees.iter().for_each(|tree| println!("{tree}"));
        }
        trees.clear();
    }
}
