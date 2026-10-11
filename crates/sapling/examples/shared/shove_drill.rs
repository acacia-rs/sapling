//! The shove, byhand and bedspawn drills of bds_command_probe: what a piston does to what stands in its way
//! (docs/research/redstone.md, "Pistons and entities"), and a lever and a button worked by hand.

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

pub const BYHAND_SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent"];

/// A lever, then a stone button, on the floor beside the actor, each with a lamp beside it and
/// dust leading to a second lamp: clicked on, and the lever off again.
pub fn byhand() -> Vec<String> {
    let mut lines: Vec<String> = ["fill ~ ~-1 ~1 ~8 ~-1 ~8 stone", "fill ~ ~ ~1 ~8 ~3 ~8 air", ""].map(String::from).into();
    for (z, switch) in [(1, "lever [\"lever_direction\"=\"up_east_west\"]"), (3, "stone_button [\"facing_direction\"=1]")] {
        lines.extend([format!("setblock ~1 ~ ~{z} {switch}"), format!("setblock ~ ~ ~{z} redstone_lamp"), format!("fill ~2 ~ ~{z} ~3 ~ ~{z} redstone_wire"), format!("setblock ~4 ~ ~{z} redstone_lamp"), String::new()]);
        lines.extend([format!("!click 1 0 {z}"), String::new(), String::new(), String::new()]);
        if z == 1 {
            lines.extend([format!("!click 1 0 {z}"), String::new(), String::new()]);
        }
    }
    lines
}

pub const BED_SHOWN: &[&str] = &[
    "UpdateBlock", "SetSpawnPosition", "Text", "Respawn", "MovePlayer", "DeathInfo", "PlayerAction", "Animate", "SetHealth", "LevelSoundEvent", "BlockEntityData", "InventorySlot", "PlayStatus",
];

/// The actor, in survival by day: puts a bed down beside it and clicks it, is killed and rises;
/// then the bed is taken away and it is killed again.
pub fn bedspawn() -> Vec<String> {
    let mut lines: Vec<String> = ["time set noon", "fill ~-2 ~-1 ~-2 ~6 ~-1 ~6 stone", "fill ~-2 ~ ~-2 ~6 ~3 ~6 air", "gamemode survival @s", "tp @s ~3 ~ ~3", ""].map(String::from).into();
    lines.extend(["give @s bed 1", "", "!place 1 -1 0", "", "!click 1 0 0", "", "", "kill @s"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 6));
    lines.extend(["fill ~-3 ~ ~-3 ~3 ~1 ~3 air", "", "kill @s"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 6));
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}
