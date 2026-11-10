//! Group AJ of the fourteenth parity brief (drill3), on the drill floor. `aj3ore`: `/place feature
//! minecraft:coal_ore_feature` in the middle of a floating cube of target blocks (a block worldgen
//! never names, which sapling's placement over loaded columns read as stone) and of stone.

use super::creatures::{floor, waits, with};

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "aj3ore" => (ores(), &["CommandOutput", "UpdateBlock", "UpdateSubchunkBlocks"]),
        _ => return None,
    })
}

fn ores() -> Vec<String> {
    let mut lines = with(floor(), &["tp @s 16 -26 -6"]);
    for (x, block) in [(4, "target"), (20, "stone")] {
        lines.extend([format!("fill {x} -16 4 {} -8 12 {block}", x + 8), String::new()]);
    }
    waits(&mut lines, 2);
    for x in [8, 24] {
        lines.extend([format!("place feature minecraft:coal_ore_feature {x} -12 8"), String::new(), String::new()]);
    }
    waits(&mut lines, 3);
    lines
}
