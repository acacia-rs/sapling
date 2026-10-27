//! Drills of what blocks do when what holds them up goes: levers, vines, wall banners and beds.

/// Packets shown of these drills.
pub const HOLD_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "AddItemEntity", "LevelEvent", "BlockEntityData"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "holds" => holds(),
            _ => return None,
        },
        HOLD_SHOWN,
    ))
}

fn floor() -> Vec<String> {
    ["kill @e[type=!player]", "gamerule domobspawning false", "gamemode creative @s", "tp @s 0.5 ~ 0.5 -90 0", "fill ~-2 ~-1 ~-6 ~10 ~-1 ~6 stone", "fill ~-2 ~ ~-6 ~10 ~6 ~6 air", ""].map(String::from).into()
}

/// Each support set to air by `/setblock`, with a second each to settle: a lever on a stone's
/// east face; a vine on a stone's east face with a vine hanging under it; a vine under a stone;
/// a wall banner on a stone's south face; a bed's foot, then a bed's head; a bed's head broken by
/// the actor in survival.
pub fn holds() -> Vec<String> {
    let mut lines = floor();
    let cases: [(&[&str], &str); 7] = [
        (&["setblock ~3 ~1 ~-4 stone", "setblock ~4 ~1 ~-4 lever [\"lever_direction\"=\"east\"]"], "setblock ~3 ~1 ~-4 air"),
        (&["setblock ~3 ~2 ~-2 stone", "setblock ~4 ~2 ~-2 vine [\"vine_direction_bits\"=2]", "setblock ~4 ~1 ~-2 vine [\"vine_direction_bits\"=2]"], "setblock ~3 ~2 ~-2 air"),
        (&["setblock ~4 ~3 ~0 stone", "setblock ~4 ~2 ~0 vine [\"vine_direction_bits\"=1]"], "setblock ~4 ~3 ~0 air"),
        (&["setblock ~3 ~1 ~2 stone", "setblock ~3 ~1 ~3 wall_banner [\"facing_direction\"=3]"], "setblock ~3 ~1 ~2 air"),
        (&["setblock ~7 ~ ~-4 bed [\"direction\"=0,\"head_piece_bit\"=false]", "setblock ~7 ~ ~-3 bed [\"direction\"=0,\"head_piece_bit\"=true]"], "setblock ~7 ~ ~-4 air"),
        (&["setblock ~7 ~ ~0 bed [\"direction\"=0,\"head_piece_bit\"=false]", "setblock ~7 ~ ~1 bed [\"direction\"=0,\"head_piece_bit\"=true]"], "setblock ~7 ~ ~1 air"),
        (&["gamemode survival @s", "setblock ~1 ~ ~4 bed [\"direction\"=0,\"head_piece_bit\"=false]", "setblock ~1 ~ ~5 bed [\"direction\"=0,\"head_piece_bit\"=true]"], "!break 1 0 4"),
    ];
    for (setup, cut) in cases {
        lines.extend(setup.iter().map(|line| line.to_string()));
        lines.extend([String::new(), cut.to_owned(), String::new(), String::new()]);
    }
    lines.extend(["gamemode creative @s", "fill ~-2 ~ ~-6 ~10 ~6 ~6 air", "kill @e[type=item]", ""].map(String::from));
    lines
}
