//! The thirteenth wave's group N drills (research/parity/THIRTEENTH-BRIEF.md): `/place`'s
//! `keepJigsaws`, `ignoreStartHeight` and `liquidSettings` (`n13jig`, `n13liq`), a scoreboard
//! holder leaving and coming back (`n13off`), and REMAINING3's player, item, command and network
//! lines (`n13play`, `n13items`, `n13more`, `n13orb`, `n13net`, `n13names`, `n13sheet`).

use super::creatures::{floor, with};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "n13jig" => (jigsaws(), JIG_SHOWN),
        "n13liq" => (liquids(), JIG_SHOWN),
        "n13off" => (offline(), OFF_SHOWN),
        "n13play" => (player(), PLAY_SHOWN),
        "n13items" => (items(), ITEMS_SHOWN),
        "n13more" => (more(), MORE_SHOWN),
        "n13orb" => (orbs(), MORE_SHOWN),
        "n13net" => (wire(), NET_SHOWN),
        "n13names" => (names(), NAMES_SHOWN),
        "n13sheet" => (sheet(), SHEET_SHOWN),
        _ => return None,
    })
}

const PLAY_SHOWN: &[&str] = &["UpdateAttributes", "SetHealth", "EntityEvent", "LevelSoundEvent", "LevelEvent", "InventorySlot", "MobArmorEquipment", "CommandOutput", "ChangeDimension"];

/// Exhaustion from a fall, a zombie's hits, a slow and a quick break and fire; a sword's and a
/// pickaxe's wear from hits and a break; leather armour breaking; a bed going off in the Nether.
fn player() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "effect @s clear", "", "tp @s 0.5 -21 0.5", "", "", ""]);
    // The zombie throws the actor; each step after it stands it back where its offsets count from.
    let home = "tp @s 0.5 -26 0.5 -90 0";
    lines.extend(["difficulty easy", "summon zombie 2.5 -26 0.5", "", "", "", "", "kill @e[type=zombie]", home, ""].map(String::from));
    lines.extend(["setblock 2 -26 0 dirt", "!break 2 0 0", "", "setblock 2 -26 0 short_grass", "!break 2 0 0", "", "setblock 0 -26 0 fire", "", "", "", "setblock 0 -26 0 air", home, ""].map(String::from));
    for tool in ["iron_sword", "iron_pickaxe"] {
        lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {tool}"), "summon cow 1.5 -26 0.5".into(), home.into(), "!hit cow".into(), "".into(), home.into(), "!hit cow".into(), "".into(), "kill @e[type=cow]".into()]);
        lines.extend([home.into(), "setblock 2 -26 0 dirt".into(), "!break 2 0 0".into(), "".into()]);
    }
    lines.extend(["effect @s instant_health 1 5 true", "replaceitem entity @s slot.armor.chest 0 leather_chestplate 1 79", "", "damage @s 4 entity_attack", "", ""].map(String::from));
    lines.extend(["effect @s fire_resistance 30 0 true", "effect @s resistance 30 2 true", "effect @s instant_health 1 5 true", "execute in nether run tp @s 0.5 100 0.5", "", "", ""].map(String::from));
    lines.extend(["fill -3 99 -3 4 104 3 air", "fill -3 99 -3 4 99 3 stone", "tp @s 0.5 100 0.5", "setblock 2 100 0 bed", "", "!sleep 2 0 0", "", "", ""].map(String::from));
    with(lines, &["execute in overworld run tp @s 0.5 -26 0.5", "", "", "gamemode creative @s", "effect @s clear", "clear @s", "kill @e[type=!player]", ""])
}

const MORE_SHOWN: &[&str] = &["UpdateAttributes", "EntityEvent", "LevelEvent", "AddEntity", "RemoveEntity", "CommandOutput", "ChangeDimension"];

/// A Nether bed going off 4 blocks from the actor on each difficulty, with no armour and
/// Resistance IV; then an orb summoned on the actor alone in survival, the watcher far off.
fn more() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "effect @s fire_resistance 120 0 true", "effect @s resistance 120 3 true", "execute in nether run tp @s 0.5 100 0.5", "", "", ""]);
    for difficulty in ["easy", "normal", "hard"] {
        lines.extend([format!("difficulty {difficulty}"), "effect @s instant_health 1 5 true".into(), "fill -3 99 -3 7 104 3 air".into(), "fill -3 99 -3 7 99 3 stone".into()]);
        lines.extend(["tp @s 0.5 100 0.5 -90 0", "setblock 4 100 0 bed", "", "!sleep 4 0 0", "", "", ""].map(String::from));
    }
    lines.extend(["execute in overworld run tp @s 0.5 -26 0.5", "effect @s clear", "difficulty easy", "tp CmdWatcher 20.5 -26 0.5", "", "", "summon xp_orb 0.5 -26 0.5", "", "", "", "", ""].map(String::from));
    with(lines, &["tp CmdWatcher 0.5 -26 0.5", "gamemode creative @s", "kill @e[type=!player]", ""])
}

/// Orbs summoned on the actor alone: three in survival, one in creative, one in adventure.
fn orbs() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 20.5 -26 0.5", "gamemode survival @s", ""]);
    for mode in ["survival", "survival", "survival", "creative", "adventure"] {
        lines.extend([format!("gamemode {mode} @s"), "summon xp_orb 0.5 -26 0.5".into(), "".into(), "".into(), "".into(), "".into()]);
    }
    with(lines, &["tp CmdWatcher 0.5 -26 0.5", "gamemode creative @s", "kill @e[type=!player]", ""])
}

const NET_SHOWN: &[&str] = &["AddEntity", "AddItemEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "MotionPredictionHints", "UpdateBlock", "UpdateBlockSynced", "LevelEvent", "LevelSoundEvent", "SpawnParticleEffect"];

/// Sand falling five blocks; a thrown item; an orb homing from four blocks; water poured in the
/// Nether.
fn wire() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "fill 3 -32 0 3 -32 0 stone", "fill 3 -31 0 3 -27 0 air", ""]);
    lines.extend(["setblock 3 -24 0 sand", "", "", "", "", "", "", "", "", "fill 3 -32 0 3 -24 0 stone", ""].map(String::from));
    lines.extend(["replaceitem entity @s slot.hotbar 0 dirt 1", "", "!drop 0 1", "", "", "", "", "", "kill @e[type=item]", ""].map(String::from));
    lines.extend(["summon xp_orb 0.5 -26 4.5", "", "", "", "", "", "", ""].map(String::from));
    lines.extend(["gamemode creative @s", "execute in nether run tp @s 0.5 100 0.5 -90 0", "", "", "", "fill -1 99 -1 3 102 1 air", "fill -1 99 -1 3 99 1 stone", "tp @s 0.5 100 0.5 -90 0", "", "replaceitem entity @s slot.weapon.mainhand 0 water_bucket", "", "!click 2 -1 0 up", "", "", ""].map(String::from));
    with(lines, &["execute in overworld run tp @s 0.5 -26 0.5", "", "clear @s", "kill @e[type=!player]", ""])
}

const SHEET_SHOWN: &[&str] = &["UpdateSubchunkBlocks", "UpdateBlock"];

/// A portal sheet over two chunks and two sections, its frame broken at the near side low and at
/// the far top.
fn sheet() -> Vec<String> {
    let mut lines = floor();
    for gap in ["14 -20 3", "17 -12 3"] {
        lines.extend(["fill 14 -27 3 17 -12 3 obsidian", "fill 15 -26 3 16 -13 3 portal [\"portal_axis\"=\"x\"]", "", ""].map(String::from));
        lines.extend([format!("setblock {gap} air"), String::new(), String::new(), String::new()]);
    }
    with(lines, &["fill 14 -27 3 17 -12 3 air", "fill 14 -27 3 17 -27 3 stone", ""])
}

const NAMES_SHOWN: &[&str] = &["CommandOutput"];

/// Old block names set on dirt and read back by a `testforblock` that fails; old item names given.
fn names() -> Vec<String> {
    const BLOCKS: [&str; 24] = ["double_plant", "coral_fan_dead", "skull", "wood", "brick_block", "hard_stained_glass", "hard_stained_glass_pane", "silver_glazed_terracotta", "invisibleBedrock", "lit_pumpkin", "magma", "web", "snow_layer", "melon_block", "golden_rail", "end_bricks", "quartz_ore", "mob_spawner", "trapdoor", "wooden_button", "wooden_pressure_plate", "fence_gate", "prismarine", "sponge"];
    const ITEMS: [&str; 14] = ["scute", "horsearmorgold", "horsearmordiamond", "lodestonecompass", "wooden_door", "netherbrick", "frame", "glow_frame", "spawn_egg", "banner", "bed", "record_cat", "map", "camera"];
    let mut lines = with(floor(), &["fill 1 -27 -1 3 -25 1 air", "setblock 2 -27 0 dirt", ""]);
    for block in BLOCKS {
        lines.extend([format!("setblock 2 -26 0 {block}"), "testforblock 2 -26 0 air".into(), "setblock 2 -25 0 air".into(), "setblock 2 -26 0 air".into()]);
    }
    for item in ITEMS {
        lines.extend([format!("give @s {item}"), "clear @s".into()]);
    }
    with(lines, &["fill 1 -27 -1 3 -27 1 stone", ""])
}

const ITEMS_SHOWN: &[&str] = &["LevelSoundEvent", "InventorySlot", "InventoryContent", "MobArmorEquipment", "AddItemEntity", "RemoveEntity", "TakeItemEntity", "AddEntity", "UpdateAttributes", "SetEntityData", "CommandOutput"];

/// Equip sounds of four pieces; donning over a worn piece, an elytra over a chestplate and
/// over binding; a shulker box broken empty and full; netherite and dirt dropped into lava; an
/// orb on a spectator, then on a survival player.
fn items() -> Vec<String> {
    let mut lines = floor();
    for (piece, slot) in [("chainmail_chestplate", "chest"), ("golden_helmet", "head"), ("turtle_helmet", "head"), ("elytra", "chest")] {
        lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {piece}"), "".into(), "!use".into(), "".into(), format!("replaceitem entity @s slot.armor.{slot} 0 air")]);
    }
    for (worn, held) in [("iron_chestplate", "diamond_chestplate"), ("iron_chestplate", "elytra")] {
        lines.extend([format!("replaceitem entity @s slot.armor.chest 0 {worn}"), format!("replaceitem entity @s slot.weapon.mainhand 0 {held}"), "".into(), "!use".into(), "".into(), "".into()]);
    }
    lines.extend(["replaceitem entity @s slot.armor.chest 0 air", "replaceitem entity @s slot.weapon.mainhand 0 iron_chestplate", "enchant @s binding 1", "!use", "", "replaceitem entity @s slot.weapon.mainhand 0 diamond_chestplate", "", "!use", "", "", "replaceitem entity @s slot.armor.chest 0 air", "clear @s"].map(String::from));
    lines.extend(["gamemode survival @s", "setblock 2 -26 0 red_shulker_box", "", "!break 2 0 0", "", "", "setblock 2 -26 0 red_shulker_box", "replaceitem block 2 -26 0 slot.container 0 dirt 3", "", "!break 2 0 0", "", "", "gamemode creative @s", "clear @s"].map(String::from));
    lines.extend(["fill 0 -27 2 0 -27 4 lava", "tp @s 0.5 -26 0.5 0 30", "replaceitem entity @s slot.hotbar 0 netherite_ingot 1", "replaceitem entity @s slot.hotbar 1 dirt 1", "", "!drop 0 1", "!drop 1 1", "", "", "", "", "fill 0 -27 2 0 -27 4 stone"].map(String::from));
    lines.extend(["gamemode spectator @s", "summon xp_orb 0.5 -26 0.5", "", "", "", "gamemode survival @s", "", "", "gamemode creative @s"].map(String::from));
    with(lines, &["clear @s", "kill @e[type=!player]", ""])
}

const JIG_SHOWN: &[&str] = &["UpdateSubchunkBlocks", "UpdateBlock", "BlockEntityData", "CommandOutput"];

/// A town centre with its jigsaws kept; trail ruins with and without `ignoreStartHeight` at two
/// heights.
fn jigsaws() -> Vec<String> {
    let mut lines = with(floor(), &["", "place jigsaw minecraft:village/plains/town_centers minecraft:bottom 1 -40 -60 0 true", "", "", ""]);
    for (at, flags) in [("0 -50 -40", ""), ("40 -50 -40", " true"), ("40 -40 40", " true"), ("0 -40 40", " false")] {
        lines.extend([format!("place structure minecraft:trail_ruins {at}{flags}"), String::new(), String::new()]);
    }
    with(lines, &["kill @e[type=!player]", ""])
}

/// A plains house placed into a water pool with each `liquidSettings` (the first try's pools were
/// past the loaded chunks).
fn liquids() -> Vec<String> {
    let mut lines = with(floor(), &[""]);
    for (x, liquids) in [(-18, "apply_waterlogging"), (18, "ignore_waterlogging")] {
        lines.extend([format!("fill {} -60 10 {} -50 34 water", x - 12, x + 12), String::new()]);
        lines.extend([format!("place jigsaw minecraft:village/plains/houses minecraft:building_entrance 1 {x} -60 22 false false {liquids}"), String::new(), String::new()]);
    }
    with(lines, &["kill @e[type=!player]", ""])
}

const OFF_SHOWN: &[&str] = &["SetScore", "SetDisplayObjective", "CommandOutput", "Text"];

/// The actor with a shown score leaves and comes back; the watcher sees the sidebar meanwhile.
fn offline() -> Vec<String> {
    let setup = ["scoreboard objectives add n13 dummy", "scoreboard objectives setdisplay sidebar n13", "scoreboard players set @s n13 5", "scoreboard players set fake n13 2", "summon pig 3 -26 3", "scoreboard players set @e[type=pig] n13 4", ""];
    let mut lines = with(floor(), &setup);
    lines.extend(["!rejoin", "", "", "scoreboard players list", "scoreboard players list *", "scoreboard players add @s n13 1", "scoreboard players list @s", ""].map(String::from));
    with(lines, &["scoreboard objectives remove n13", "kill @e[type=!player]", ""])
}
