//! Drills of rails and power: how far a signal runs along powered and activator rails, and how a
//! plain rail at a junction turns with a signal.

/// Packets shown of these drills.
pub const RAIL_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "rails" => rails(),
            _ => return None,
        },
        RAIL_SHOWN,
    ))
}

fn floor() -> Vec<String> {
    ["kill @e[type=!player]", "gamerule domobspawning false", "gamemode creative @s", "tp @s 0.5 ~ 0.5 -90 0", "fill ~-2 ~-1 ~-8 ~16 ~-1 ~8 stone", "fill ~-2 ~ ~-8 ~16 ~4 ~8 air", ""].map(String::from).into()
}

/// A line of twelve powered rails, then of twelve activator rails, running east: a block of
/// redstone put beside the first and taken away; then beside the sixth. A line of rails rising a
/// step halfway. Then a plain rail at a T of rails, a block of redstone put beside it and taken
/// away.
pub fn rails() -> Vec<String> {
    let mut lines = floor();
    for (rail, z) in [("golden_rail", -6), ("activator_rail", -3)] {
        lines.push(format!("fill ~1 ~ ~{z} ~12 ~ ~{z} {rail} [\"rail_direction\"=1]"));
        lines.push(String::new());
        for x in [1, 6] {
            lines.extend([format!("setblock ~{x} ~ ~{} redstone_block", z - 1), String::new(), String::new(), format!("setblock ~{x} ~ ~{} air", z - 1), String::new(), String::new()]);
        }
    }
    // Rising: rails at ~1..~5 on the floor, a stone step and rails at ~6..~10 one up.
    lines.extend([
        "fill ~1 ~ ~0 ~5 ~ ~0 golden_rail [\"rail_direction\"=1]".to_owned(),
        "fill ~6 ~ ~0 ~10 ~ ~0 stone".to_owned(),
        "fill ~6 ~1 ~0 ~10 ~1 ~0 golden_rail [\"rail_direction\"=1]".to_owned(),
        String::new(),
        "setblock ~1 ~ ~-1 redstone_block".to_owned(),
        String::new(),
        String::new(),
        "setblock ~1 ~ ~-1 air".to_owned(),
        String::new(),
    ]);
    // The T: rails west, east and south of a rail at ~2 ~ ~2, placed by the actor last so it joins them.
    lines.extend([
        "fill ~1 ~ ~0 ~10 ~1 ~0 air".to_owned(),
        "setblock ~1 ~ ~2 rail [\"rail_direction\"=1]".to_owned(),
        "setblock ~3 ~ ~2 rail [\"rail_direction\"=1]".to_owned(),
        "setblock ~2 ~ ~3 rail [\"rail_direction\"=0]".to_owned(),
        "replaceitem entity @s slot.weapon.mainhand 0 rail".to_owned(),
        "!place 2 -1 2".to_owned(),
        String::new(),
        "setblock ~2 ~ ~1 redstone_block".to_owned(),
        String::new(),
        String::new(),
        "setblock ~2 ~ ~1 air".to_owned(),
        String::new(),
        String::new(),
        "setblock ~2 ~-1 ~2 redstone_block".to_owned(),
        String::new(),
        String::new(),
        "setblock ~2 ~-1 ~2 stone".to_owned(),
        String::new(),
        String::new(),
        "fill ~-2 ~ ~-8 ~16 ~4 ~8 air".to_owned(),
        "clear @s".to_owned(),
        String::new(),
    ]);
    lines
}
