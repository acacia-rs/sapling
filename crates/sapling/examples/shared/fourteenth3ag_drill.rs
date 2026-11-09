//! Group AG of the fourteenth parity brief (drill3), on the drill floor (stone at y -27). `ag3pin`:
//! y3pin's off-middle rows again, with armour stands put where the cows' sides were (z 0.1 and 0.9,
//! a cow's edge 0.15 into the next row) and at the cows' own places, so turns cannot decide it,
//! then the cows again.

use super::creatures::{floor, waits, with};

const MOVES: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "UpdateBlock", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ag3pin" => (pinned(), MOVES),
        _ => return None,
    })
}

/// Rows two apart (z), each a piston at x 2 facing east, obsidian at x 4, and who stands at x 3.5,
/// that far into its row.
const ROWS: [(i32, &str, f32); 6] = [(-7, "armor_stand", 0.1), (-5, "armor_stand", 0.9), (-3, "armor_stand", 0.3), (-1, "armor_stand", 0.7), (1, "cow", 0.3), (3, "cow", 0.7)];

fn pinned() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "tp @s -6.5 -26 -6.5 -90 0"]);
    for (z, who, off) in ROWS {
        lines.extend([format!("setblock 2 -26 {z} piston [\"facing_direction\"=4]"), format!("setblock 4 -26 {z} obsidian"), format!("summon {who} 3.5 -26 {}", z as f32 + off)]);
    }
    lines = with(lines, &["effect @e[type=cow] slowness 100 255 true", ""]);
    for (z, ..) in ROWS {
        lines.extend([format!("setblock 2 -25 {z} redstone_block"), String::new()]);
    }
    waits(&mut lines, 4);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air"])
}
