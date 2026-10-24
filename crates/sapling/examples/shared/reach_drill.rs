//! The reach-close drill of bds_command_probe: whether, and how far off, the server shuts a chest
//! screen its player walked away from, and that it does when the chest goes
//! (docs/research/assumptions-checked.md 2).

pub const REACH_SHOWN: &[&str] = &["ContainerOpen", "ContainerClose"];

pub fn reach_close() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~-2 ~-1 ~-2 ~70 ~-1 ~2 stone", "fill ~-2 ~ ~-2 ~70 ~4 ~2 air", "gamemode survival @s", "setblock ~1 ~ ~ chest", ""].map(String::from).into();
    for away in [8, 16, 64] {
        lines.extend(["!click 1 0 0".into(), String::new(), format!("tp @s ~{away} ~ ~"), String::new(), String::new(), String::new(), format!("tp @s ~-{away} ~ ~"), String::new()]);
    }
    lines.extend(["!click 1 0 0", "", "setblock ~1 ~ ~ air", "", "", "gamemode creative @s", ""].map(String::from));
    lines
}

/// The lids drill: whether a double chest opens with a block over its other half, the one clicked,
/// a slab over the other, or nothing (NEXT.md 3); each chest goes after, shutting what opened.
pub fn lids() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~-2 ~-1 ~-2 ~4 ~-1 ~2 stone", "fill ~-2 ~ ~-2 ~4 ~4 ~2 air", "gamemode survival @s", ""].map(String::from).into();
    for (over, click) in [("~2 ~1 ~ stone", 1), ("~1 ~1 ~ stone", 2), ("~2 ~1 ~ oak_slab", 1), ("~3 ~1 ~ stone", 1)] {
        lines.extend(["setblock ~1 ~ ~ chest".into(), "setblock ~2 ~ ~ chest".into(), format!("setblock {over}"), String::new(), format!("!click {click} 0 0"), String::new(), String::new()]);
        lines.extend(["fill ~1 ~ ~ ~3 ~1 ~ air", "", ""].map(String::from));
    }
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}

/// The trapdoors drill: the actor in creative puts three trapdoors on the floor a block east of it,
/// looking each of four ways, to see the state each takes (the placing drill named them wrongly).
pub fn trapdoors() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~-2 ~-1 ~-2 ~4 ~-1 ~30 stone", "fill ~-2 ~ ~-2 ~4 ~3 ~30 air", "gamemode creative @s", "clear @s", ""].map(String::from).into();
    for item in ["trapdoor", "spruce_trapdoor", "iron_trapdoor"] {
        for yaw in [0, 90, 180, -90] {
            lines.extend([format!("tp @s ~ ~ ~2 {yaw} 0"), "fill ~1 ~ ~ ~1 ~1 ~ air".into(), format!("replaceitem entity @s slot.weapon.mainhand 0 {item}"), "!place 1 -1 0".into()]);
        }
    }
    lines.extend(["clear @s", ""].map(String::from));
    lines
}

/// The place-faces drill: stone put by hand on the top, a side and the bottom of a block, to see
/// which cells the server shows its placer again (NEXT.md 3).
pub fn place_faces() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~-2 ~-1 ~-2 ~4 ~-1 ~2 stone", "fill ~-2 ~ ~-2 ~4 ~4 ~2 air", "gamemode creative @s", "replaceitem entity @s slot.weapon.mainhand 0 stone", ""].map(String::from).into();
    for (anchor, click) in [("~1 ~-1 ~", "1 -1 0 up"), ("~2 ~ ~", "2 0 0 west"), ("~1 ~ ~-1", "1 0 -1 south"), ("~1 ~2 ~", "1 2 0 down")] {
        lines.extend([format!("setblock {anchor} gold_block"), String::new(), format!("!place {click}"), String::new(), String::new(), "fill ~1 ~ ~-1 ~2 ~2 ~1 air".into(), "fill ~1 ~-1 ~ ~1 ~-1 ~ stone".into(), String::new()]);
    }
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}

pub const HELD_USE_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "ContainerOpen", "ContainerClose", "BlockEvent", "LevelSoundEvent"];

/// The held-use drill: a lever, a chest and a door clicked holding dirt, then by an empty hand;
/// whether the server shows the blocks again after a use with a block in hand (NEXT.md 3).
pub fn held_use() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~-2 ~-1 ~-2 ~4 ~-1 ~2 stone", "fill ~-2 ~ ~-2 ~4 ~3 ~2 air", "gamemode survival @s", ""].map(String::from).into();
    for held in ["dirt", "air"] {
        lines.push(format!("replaceitem entity @s slot.weapon.mainhand 0 {held}"));
        for block in ["lever", "chest", "wooden_door"] {
            lines.extend([format!("setblock ~1 ~ ~ {block}"), String::new(), "!click 1 0 0".into(), String::new(), String::new(), "fill ~1 ~ ~ ~1 ~1 ~ air".into(), String::new()]);
        }
    }
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}

/// The shut drill: a chest, a barrel and an ender chest opened and closed by the screen, each a
/// few seconds apart, to hear what closing them sounds like and see when an open lid is shown
/// again (NEXT.md 3).
pub fn shut() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~-2 ~-1 ~-2 ~4 ~-1 ~2 stone", "fill ~-2 ~ ~-2 ~4 ~3 ~2 air", "gamemode creative @s", ""].map(String::from).into();
    for block in ["chest", "barrel", "ender_chest"] {
        lines.extend([format!("setblock ~1 ~ ~ {block}"), String::new(), "!click 1 0 0".into()]);
        lines.extend(std::iter::repeat_n(String::new(), 6));
        lines.extend(["!close".into(), String::new(), String::new(), "setblock ~1 ~ ~ air".into(), String::new()]);
    }
    lines
}
