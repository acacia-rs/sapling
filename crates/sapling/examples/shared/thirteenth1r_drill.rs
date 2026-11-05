//! The thirteenth wave's group R drills (research/parity/THIRTEENTH-BRIEF.md): REMAINING3's
//! effects (`r1fx`), enchantment physics (`r1ench`, `r1swim`), mobs in water (`r1water`) and the
//! item lines (thirteenth1r_items.rs).

#[path = "thirteenth1r_items.rs"]
mod items;

use crate::creatures::{floor, waits, with};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "r1fx" => (effects(), FX_SHOWN),
        "r1ench" => (enchants(), ENCH_SHOWN),
        "r1swim" => (swim(), ENCH_SHOWN),
        "r1water" => (water(), WATER_SHOWN),
        "r1locate" => (locate(), &["CommandOutput"]),
        _ => return items::drill(name),
    })
}

/// On a generated world of seed 12345 (BDS level-type DEFAULT; sapling `generated`): `/locate biome`
/// from over the origin, where the nearer hits are in loaded columns.
fn locate() -> Vec<String> {
    const BIOMES: [&str; 24] = [
        "plains", "sunflower_plains", "forest", "flower_forest", "birch_forest", "dark_forest", "river", "ocean", "deep_ocean", "lukewarm_ocean", "cold_ocean", "taiga", "savanna", "jungle", "swamp", "beach",
        "stony_shore", "meadow", "windswept_hills", "snowy_plains", "lush_caves", "dripstone_caves", "deep_dark", "cherry_grove",
    ];
    let mut lines = vec!["gamemode spectator @s".to_owned(), "tp @s 0 120 0".to_owned()];
    waits(&mut lines, 5);
    lines.extend(BIOMES.map(|biome| format!("locate biome {biome}")));
    lines
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

/// Held, enchanted with each of `enchants`, then worn.
fn worn(lines: &mut Vec<String>, piece: &str, enchants: &[&str]) {
    lines.push(hold(piece));
    lines.extend(enchants.iter().map(|enchant| format!("enchant @s {enchant}")));
    lines.extend(["!use", ""].map(String::from));
}

const FX_SHOWN: &[&str] = &["MobEffect", "UpdateAttributes", "SetEntityData", "SetHealth", "Respawn", "CommandOutput"];

/// In survival: speed then powder snow's freeze; Soul Speed boots on soul sand with speed and
/// slowness given after; clears at several points of short effects; effects at a death and rise;
/// absorption taking a hit.
fn effects() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "effect @s speed 60 1 true", "", "setblock 2 -26 0 powder_snow", "tp @s 2.5 -26 0.5"]);
    waits(&mut lines, 9);
    lines.extend(["tp @s 0.5 -26 0.5", "setblock 2 -26 0 air", "", "", "", "effect @s clear", ""].map(String::from));
    worn(&mut lines, "diamond_boots", &["soul_speed 3"]);
    lines.extend(["fill -2 -27 -2 3 -27 2 soul_sand", "", "", "effect @s speed 30 1 true", "", "", "effect @s slowness 30 0 true", "", "", "effect @s clear", ""].map(String::from));
    lines.extend(["fill -2 -27 -2 3 -27 2 stone", "replaceitem entity @s slot.armor.feet 0 air", ""].map(String::from));
    for (seconds, kept) in [(1, 0), (2, 1), (3, 2), (30, 0), (2, 0)] {
        lines.push(format!("effect @s speed {seconds} 0"));
        waits(&mut lines, kept);
        lines.extend(["effect @s clear", ""].map(String::from));
    }
    lines.extend(["effect @s speed 60 1", "effect @s absorption 60 1", "effect @s regeneration 60 0", "", "kill @s", "", "", "", "", ""].map(String::from));
    lines.extend(["effect @s clear", "effect @s absorption 30 1", "", "damage @s 3 entity_attack", "", "", "effect @s instant_health 1 5 true", ""].map(String::from));
    with(lines, &["gamemode creative @s", "effect @s clear", "clear @s", ""])
}

const ENCH_SHOWN: &[&str] = &["LevelEvent", "SetEntityMotion", "UpdateAttributes", "SetHealth", "EntityEvent", "MovePlayer", "CorrectPlayerMovePrediction", "CommandOutput", "InventorySlot", "MobArmorEquipment"];

/// A slowed cow hit by a plain sword and by Knockback I and II; the actor beside a TNT bare and in
/// Blast Protection IV; `/damage` of 10 by four causes bare, in diamond, and in diamond with
/// Protection IV on every piece.
fn enchants() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 12.5 -26 6.5", "summon cow 2.5 -26 0.5", "effect @e[type=cow] slowness 999 255 true", "effect @e[type=cow] regeneration 999 5 true", ""]);
    for knockback in [0, 1, 2] {
        lines.extend([hold("iron_sword"), if knockback > 0 { format!("enchant @s knockback {knockback}") } else { String::new() }]);
        lines.extend(["tp @e[type=cow] 2.5 -26 0.5", "tp @s 0.5 -26 0.5 -90 0", "", "!hit cow", "", "", ""].map(String::from));
    }
    lines.extend(["kill @e[type=cow]", "clear @s", "gamemode survival @s", "effect @s resistance 999 3 true", "effect @s health_boost 999 9 true", ""].map(String::from));
    for blast in [false, true] {
        if blast {
            worn(&mut lines, "iron_chestplate", &["blast_protection 4"]);
        }
        lines.extend(["effect @s instant_health 1 9 true", "tp @s 0.5 -26 0.5 -90 0", "", "summon tnt 2.5 -26 0.5", "", "", "", "", ""].map(String::from));
        lines.extend(["fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -16 8 air", ""].map(String::from));
    }
    lines.extend(["clear @s", "effect @s clear", "effect @s resistance 0 0", ""].map(String::from));
    const SUIT: [(&str, &str); 4] = [("helmet", "head"), ("chestplate", "chest"), ("leggings", "legs"), ("boots", "feet")];
    for armour in ["", "diamond", "protected"] {
        for (piece, slot) in SUIT {
            match armour {
                "diamond" => lines.push(format!("replaceitem entity @s slot.armor.{slot} 0 diamond_{piece}")),
                "protected" => {
                    lines.push(format!("replaceitem entity @s slot.armor.{slot} 0 air"));
                    worn(&mut lines, &format!("diamond_{piece}"), &["protection 4"]);
                }
                _ => {}
            }
        }
        for cause in ["entity_attack", "fall", "projectile", "magic"] {
            lines.extend(["effect @s instant_health 1 9 true".into(), String::new(), format!("damage @s 10 {cause}"), String::new()]);
        }
    }
    with(lines, &["gamemode creative @s", "effect @s clear", "clear @s", "tp CmdWatcher 0.5 -26 0.5", ""])
}

/// Walks through still water bare-footed and in Depth Strider III boots (run against a strict BDS
/// to see whether it corrects the bot's own swim).
fn swim() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 12.5 -26 6.5", "gamemode survival @s", "fill -8 -26 -2 16 -25 2 water", "!look 8 1.6 0", ""]);
    for boots in [false, true] {
        if boots {
            worn(&mut lines, "diamond_boots", &["depth_strider 3"]);
        }
        lines.extend(["tp @s -6.5 -26 0.5 -90 0", "!look 8 1.6 0", "", "!walk", "", "", "", "!stop", ""].map(String::from));
    }
    with(lines, &["fill -8 -26 -2 16 -25 2 air", "gamemode creative @s", "clear @s", "tp CmdWatcher 0.5 -26 0.5", ""])
}

const WATER_SHOWN: &[&str] = &["AddEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "SetEntityData", "EntityEvent", "RemoveEntity"];

/// A pool three deep whose rim stands a block over the water: a cow, a pig, a zombie and a
/// skeleton dropped in, then a cow and a zombie in a one-wide shaft of water two and three deep.
fn water() -> Vec<String> {
    let mut lines = with(floor(), &["tp @s -5.5 -26 -5.5", "tp CmdWatcher -6.5 -26 -5.5", "fill 1 -31 -3 9 -27 3 stone", "fill 2 -30 -2 8 -28 2 water", ""]);
    lines.extend(["summon cow 3.5 -28 0.5", "summon pig 5.5 -28 0.5", "summon zombie 7.5 -29 -1.5", "summon skeleton 7.5 -29 1.5"].map(String::from));
    waits(&mut lines, 14);
    lines.extend(["kill @e[type=!player]", "fill 1 -31 -3 9 -27 3 stone", "fill 12 -30 -6 14 -26 -4 stone", "fill 13 -29 -5 13 -28 -5 water", "fill 12 -30 4 14 -26 6 stone", "fill 13 -29 5 13 -27 5 water", ""].map(String::from));
    lines.extend(["summon cow 13.5 -28 -4.5", "summon zombie 13.5 -27 5.5"].map(String::from));
    waits(&mut lines, 14);
    with(lines, &["kill @e[type=!player]", "fill 12 -30 -6 14 -26 6 air", "fill 12 -27 -6 14 -27 6 stone", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}
