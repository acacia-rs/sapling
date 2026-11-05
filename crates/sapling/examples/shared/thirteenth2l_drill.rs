//! The thirteenth wave's group L drills (research/parity/THIRTEENTH-BRIEF.md): the random tick's pace at a
//! rule high enough that a cell comes up more than once a tick (`l2pace`).

use super::creatures::{floor, waits};

const SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "CommandOutput"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "l2pace" => (pace(), SHOWN),
        "l2farm" => (farmland(), SHOWN),
        _ => return None,
    })
}

/// Farmland with no water near, at `randomtickspeed` 20 for 40 s then 200 for 10 s: 64 dry cells in chunk 0,0
/// (each turns to dirt at its first random tick) and 64 wet ones under wheat in chunk 0,-1 (a step drier
/// each random tick, kept by the crop), both chunks' highest block in section 2.
fn farmland() -> Vec<String> {
    let mut lines = floor();
    for rule in [20, 200] {
        lines.extend(["fill 1 -27 1 8 -27 8 farmland [\"moisturized_amount\"=0]", "fill 1 -27 -8 8 -27 -1 farmland [\"moisturized_amount\"=7]", "fill 1 -26 -8 8 -26 -1 wheat", ""].map(String::from));
        lines.push(format!("gamerule randomtickspeed {rule}"));
        waits(&mut lines, if rule == 20 { 40 } else { 10 });
        lines.extend(["gamerule randomtickspeed 0", "fill 1 -26 -8 8 -26 8 air", ""].map(String::from));
    }
    lines.push("gamerule randomtickspeed 1".to_string());
    lines
}

/// 91 stalactites hanging from dripstone blocks under a water pool in chunk 0,0 (its highest block in
/// section 2): three rounds of 6 s at `randomtickspeed` 4096, then one of 20 s at 256. Each root's
/// first growth (a piece under y -21) times its random ticks: 0.0114 of them grow.
fn pace() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["fill 0 -20 0 14 -18 8 stone", "fill 1 -19 1 13 -19 7 water", "fill 1 -20 1 13 -20 7 dripstone_block", ""].map(String::from));
    for rule in [4096, 4096, 4096, 256] {
        lines.extend(["fill 1 -26 1 13 -21 7 air", "fill 1 -21 1 13 -21 7 pointed_dripstone [\"dripstone_thickness\"=\"tip\",\"hanging\"=true]", ""].map(String::from));
        lines.push(format!("gamerule randomtickspeed {rule}"));
        waits(&mut lines, if rule > 256 { 6 } else { 20 });
        lines.extend(["gamerule randomtickspeed 0", ""].map(String::from));
    }
    lines.push("gamerule randomtickspeed 1".to_string());
    lines
}
