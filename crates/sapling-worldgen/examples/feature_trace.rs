//! `feature_trace <seed> <chunk x> <chunk z> [substring]`: the decoration trace of one source chunk
//! (each rule try with its position, result and draw range in the rule's placing generator, each
//! tree attempt under it), keeping the lines that contain the substring. Beside
//! `tree_draws` (BDS's trunks by draw index) it names the first try that desyncs.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, cx, cz, rest @ ..] = &args[..] else { panic!("usage: feature_trace <seed> <cx> <cz> [substring]") };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    // The trace holds the earlier phases' sources too.
    let own = format!("({cx}, {cz}) ");
    let mut trees = Vec::new();
    for line in generator.source_feature_trace(cx, cz) {
        // A try's tree lines come before its own line.
        if line.starts_with(' ') {
            trees.push(line);
            continue;
        }
        if line.starts_with(&own) && rest.first().is_none_or(|want| line.contains(want.as_str())) {
            println!("{line}");
            trees.iter().for_each(|tree| println!("{tree}"));
        }
        trees.clear();
    }
}
