//! The fourteenth wave's group AD drill (research/parity/FOURTEENTH-BRIEF.md), on a generated world (seed
//! 20261003). `adplace`: y3place's trail ruins, the actor then visiting the columns they reach out of its
//! sight for BDS to write what it deferred, and abandoned camps in sight last, for the world folder to hold
//! what their chests and barrels were given.

use super::creatures::{waits, with};

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "adplace" => (placed(), &["CommandOutput", "UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData"]),
        _ => return None,
    })
}

fn placed() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 15);
    lines = with(lines, &["gamerule domobspawning false", "gamemode creative @s", "tp @s 30 120 210"]);
    waits(&mut lines, 10);
    for at in ["70 70 210 true", "-10 70 210 false", "30 70 250 false false true", "30 70 170 false false false", "70 40 170 true"] {
        lines.extend([format!("place structure minecraft:trail_ruins {at}"), String::new(), String::new()]);
    }
    for at in ["90 120 170", "30 120 270", "-30 120 210", "30 120 210"] {
        lines.push(format!("tp @s {at}"));
        waits(&mut lines, 8);
    }
    for at in ["10 70 205", "50 70 215", "30 70 195", "45 70 235", "15 70 230"] {
        lines.extend([format!("place structure minecraft:abandoned_camp_forest {at}"), String::new(), String::new()]);
    }
    waits(&mut lines, 5);
    lines
}
