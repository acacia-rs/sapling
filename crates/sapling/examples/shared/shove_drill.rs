//! The shove drill of bds_command_probe: what a piston does to what stands in its way
//! (docs/research/redstone.md, "Pistons and entities").

pub const SHOVE_SHOWN: &[&str] = &["AddEntity", "AddItemEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "UpdateBlock", "BlockEntityData", "EntityEvent", "UpdateAttributes"];

/// Pistons pushing east on a stone floor, each powered for two seconds: an armour stand before
/// the arm; one behind a stone the arm pushes; one before a sticky piston; one on a piston that
/// pushes up; one against a wall of obsidian; and an item before the arm.
pub fn shove() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamerule domobspawning false", "fill ~2 ~-1 ~3 ~12 ~-1 ~22 stone", "fill ~2 ~ ~3 ~12 ~5 ~22 air", ""].map(String::from).into();
    let east = "piston [\"facing_direction\"=4]";
    let rows: [(i32, Vec<String>, &str); 6] = [
        (5, vec![format!("setblock ~4 ~ ~5 {east}")], "summon armor_stand ~5 ~ ~5"),
        (8, vec![format!("setblock ~4 ~ ~8 {east}"), "setblock ~5 ~ ~8 stone".into()], "summon armor_stand ~6 ~ ~8"),
        (11, vec!["setblock ~4 ~ ~11 sticky_piston [\"facing_direction\"=4]".into()], "summon armor_stand ~5 ~ ~11"),
        (14, vec!["setblock ~4 ~ ~14 piston [\"facing_direction\"=1]".into()], "summon armor_stand ~4 ~1 ~14"),
        (17, vec![format!("setblock ~4 ~ ~17 {east}"), "fill ~6 ~ ~17 ~6 ~1 ~17 obsidian".into()], "summon armor_stand ~5 ~ ~17"),
        (20, vec![format!("setblock ~4 ~ ~20 {east}"), "setblock ~5 ~2 ~20 dirt".into()], "setblock ~5 ~2 ~20 air destroy"),
    ];
    for (z, built, stands) in rows {
        lines.extend(built);
        lines.extend([stands.to_owned(), String::new(), String::new()]);
        // From the side for the one that pushes up, from above for the rest.
        let power = if z == 14 { format!("~3 ~ ~{z}") } else { format!("~4 ~1 ~{z}") };
        lines.extend([format!("setblock {power} redstone_block"), String::new(), String::new(), format!("setblock {power} air"), String::new(), String::new()]);
    }
    lines.extend(["kill @e[type=!player]", ""].map(String::from));
    lines
}
