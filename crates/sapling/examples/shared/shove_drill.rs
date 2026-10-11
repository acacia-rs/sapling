//! The shove, byhand, bedspawn, boom and shoot drills of bds_command_probe: what a piston does to what stands in its way
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

pub const BOOM_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "LevelSoundEvent", "LevelEvent", "SetEntityMotion", "RemoveEntity", "AddEntity", "AddItemEntity", "MoveEntityDelta", "UpdateBlock", "UpdateSubchunkBlocks",
    "DeathInfo",
];

/// The actor in survival on a floor of stone over dirt, a creeper 6 blocks off left to come and
/// blow up; then the same with `mobgriefing` off.
pub fn boom() -> Vec<String> {
    let mut lines: Vec<String> =
        ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "fill ~-3 ~-4 ~-12 ~12 ~-2 ~40 dirt", "fill ~-3 ~-1 ~-12 ~12 ~-1 ~40 stone", "fill ~-3 ~ ~-12 ~12 ~6 ~40 air", "gamemode survival @s", "tp @s ~6 ~ ~20", ""].map(String::from).into();
    for rule in ["true", "false"] {
        lines.extend([format!("gamerule mobgriefing {rule}"), "effect @s instant_health 1 4".into(), "summon creeper ~ ~ ~-6".into()]);
        lines.extend(std::iter::repeat_n(String::new(), 9));
    }
    lines.extend(["gamerule mobgriefing true", "gamemode creative @s", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// The actor in survival on the boom drill's floor and a skeleton 10 blocks off, left to shoot at
/// it for ten seconds; then one 4 blocks off.
pub fn shoot() -> Vec<String> {
    let mut lines: Vec<String> =
        ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "time set midnight", "fill ~-3 ~-1 ~-12 ~12 ~-1 ~40 stone", "fill ~-3 ~ ~-12 ~12 ~6 ~40 air", "gamemode survival @s", "tp @s ~6 ~ ~20", ""].map(String::from).into();
    for far in [10, 4] {
        lines.extend(["effect @s instant_health 1 4".into(), format!("summon skeleton ~ ~ ~-{far}")]);
        lines.extend(std::iter::repeat_n(String::new(), 10));
        lines.extend(["kill @e[type=!player]", "", ""].map(String::from));
    }
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}

pub const WEAR_SHOWN: &[&str] = &["UpdateAttributes", "EntityEvent", "LevelSoundEvent", "InventorySlot", "InventoryContent", "MobArmorEquipment", "PlayerArmorDamage", "RemoveEntity", "AddEntity"];

/// The actor in survival in all of iron, hit by a zombie for six seconds, then for 8 and 20 by
/// a cactus, 2 by fire and 2 by magic. A hit by an anvil, which sends the helmet anew, comes
/// before and after each, to read what the helmet lost.
pub fn wear() -> Vec<String> {
    let mut lines: Vec<String> =
        ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "time set midnight", "fill ~-3 ~-1 ~-12 ~12 ~-1 ~40 stone", "fill ~-3 ~ ~-12 ~12 ~6 ~40 air", "gamemode survival @s", "tp @s ~6 ~ ~20", ""].map(String::from).into();
    for (part, piece) in [("head", "helmet"), ("chest", "chestplate"), ("legs", "leggings"), ("feet", "boots")] {
        lines.push(format!("replaceitem entity @s slot.armor.{part} 0 iron_{piece}"));
    }
    let read = ["effect @s instant_health 1 4", "", "damage @s 2 anvil", ""].map(String::from);
    lines.extend(read.clone());
    lines.push("summon zombie ~ ~ ~-3".into());
    lines.extend(std::iter::repeat_n(String::new(), 6));
    lines.extend(["kill @e[type=!player]", ""].map(String::from));
    lines.extend(read.clone());
    for hit in ["damage @s 8 contact", "damage @s 20 contact", "damage @s 2 fire", "damage @s 2 magic"] {
        lines.extend([hit.to_owned(), String::new()]);
        lines.extend(read.clone());
    }
    lines.extend(["gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}
