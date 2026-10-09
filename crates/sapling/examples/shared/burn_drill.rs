//! The burn drill of bds_command_probe: mobs in lava and fire (docs/research/mob-damage.md).

use std::time::Duration;

pub const BURN_SHOWN: &[&str] = &["AddEntity", "AddItemEntity", "RemoveEntity", "EntityEvent", "SetEntityData", "LevelSoundEvent", "UpdateAttributes", "SetEntityMotion", "LevelEvent", "MobEffect"];
/// A second a line: an empty line only waits.
pub const BURN_SETTLE: Duration = Duration::from_millis(1000);

/// A closed stone cell around `x`, `z` with `floor` under a mob of `kind` put into it, watched `wait` seconds.
fn cell(lines: &mut Vec<String>, x: i32, z: i32, floor: &[&str], kind: &str, wait: usize) {
    lines.push(format!("fill ~{} ~ ~{} ~{} ~3 ~{} stone", x - 1, z - 1, x + 1, z + 1));
    lines.push(format!("setblock ~{x} ~1 ~{z} air"));
    lines.extend(floor.iter().map(|block| format!("setblock ~{x} ~ ~{z} {block}")));
    lines.push(format!("summon {kind} ~{x} ~1 ~{z}"));
    lines.extend(std::iter::repeat_n(String::new(), wait));
}

/// A pig and a chicken left in lava, a zombified piglin in it, a pig in fire, and an iron golem
/// taken out of lava after a second to burn, then put from lava into water.
pub fn burn() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "", "fill ~2 ~-1 ~4 ~22 ~-1 ~16 stone", "fill ~2 ~ ~4 ~22 ~6 ~16 air"].map(String::from).into();
    cell(&mut lines, 4, 6, &["lava"], "pig", 5);
    cell(&mut lines, 8, 6, &["lava"], "chicken", 4);
    cell(&mut lines, 12, 6, &["lava"], "zombie_pigman", 4);
    lines.push("kill @e[type=zombie_pigman]".into());
    cell(&mut lines, 4, 10, &["netherrack"], "cow", 0);
    lines.push("setblock ~4 ~1 ~10 fire".into());
    lines.extend(std::iter::repeat_n(String::new(), 14));
    let pool = |block: &str| format!("fill ~16 ~ ~8 ~18 ~ ~10 {block}");
    lines.extend(["fill ~15 ~ ~7 ~19 ~5 ~11 stone".into(), "fill ~16 ~1 ~8 ~18 ~4 ~10 air".into(), pool("lava"), "summon iron_golem ~17 ~1 ~9".into(), pool("stone")]);
    lines.extend(std::iter::repeat_n(String::new(), 20));
    lines.extend([pool("lava"), pool("water")]);
    lines.extend(std::iter::repeat_n(String::new(), 3));
    lines.extend([pool("stone"), "kill @e[type=iron_golem]".into(), String::new(), String::new()]);
    lines
}

/// Cows dropped from 3 to 30 blocks, one summoned under a low roof, one roofed over after two
/// seconds, and one shut in water.
pub fn harm() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "", "fill ~2 ~-1 ~4 ~22 ~-1 ~16 stone", "fill ~2 ~ ~4 ~22 ~6 ~16 air"].map(String::from).into();
    lines.extend(["3", "3.5", "4", "4.5", "5", "8", "14", "30"].iter().enumerate().map(|(at, height)| format!("summon cow ~{} ~{height} ~6", 3 + 2 * at)));
    lines.extend(std::iter::repeat_n(String::new(), 4));
    lines.extend(["kill @e[type=cow]".into(), String::new(), String::new()]);
    cell(&mut lines, 4, 10, &["stone"], "cow", 4);
    lines.extend(["fill ~7 ~ ~9 ~9 ~3 ~11 stone", "fill ~8 ~1 ~10 ~8 ~2 ~10 air", "summon cow ~8 ~1 ~10", "", "setblock ~8 ~2 ~10 stone", "", "", "", ""].map(String::from));
    lines.extend(["fill ~11 ~ ~9 ~13 ~3 ~11 stone", "fill ~12 ~1 ~10 ~12 ~2 ~10 water", "summon cow ~12 ~1 ~10"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 32));
    lines
}

/// Mobs of several sizes set on fire, then given water to stand in.
pub fn douse() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "", "fill ~2 ~-1 ~4 ~22 ~-1 ~16 stone", "fill ~2 ~ ~4 ~22 ~6 ~16 air"].map(String::from).into();
    for (at, kind) in ["pig", "chicken", "cow", "spider", "iron_golem"].iter().enumerate() {
        let x = 4 + 4 * at as i32;
        lines.extend([format!("fill ~{} ~ ~5 ~{} ~5 ~7 stone", x - 1, x + 1), format!("fill ~{x} ~1 ~6 ~{x} ~4 ~6 air"), format!("setblock ~{x} ~ ~6 netherrack")]);
        lines.extend([format!("summon {kind} ~{x} ~1 ~6"), format!("setblock ~{x} ~1 ~6 fire"), String::new(), format!("setblock ~{x} ~1 ~6 water"), String::new(), String::new()]);
    }
    lines.extend(["kill @e[type=!player]".into(), String::new()]);
    lines
}

pub const HIT_SHOWN: &[&str] = &[
    "AddEntity", "AddItemEntity", "RemoveEntity", "EntityEvent", "SetEntityData", "LevelSoundEvent", "UpdateAttributes", "SetEntityMotion", "MoveEntityDelta", "LevelEvent", "Animate", "MobEquipment",
];

/// The actor hits cows kept in a pit with its fist and with swords, twice at once, and to
/// death; then a cow and a pig standing free, to see them thrown. `!hit` and `!hit2` are the
/// probe's own lines: one hit, or two in a row, on the newest mob of the kind.
pub fn hit() -> Vec<String> {
    let hand = |item: &str| format!("replaceitem entity @s slot.weapon.mainhand 0 {item}");
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "", "fill ~-3 ~-4 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~6 ~8 air", "fill ~1 ~-2 ~ ~1 ~-1 ~ air"].map(String::from).into();
    lines.extend([hand("air"), "summon cow ~1 ~-2 ~".into(), String::new(), "!hit cow".into(), "!hit cow".into(), String::new()]);
    lines.extend([hand("wooden_sword"), "!hit cow".into(), String::new(), hand("diamond_sword"), "!hit cow".into(), String::new(), String::new(), String::new()]);
    lines.extend([hand("air"), "summon cow ~1 ~-2 ~".into(), String::new(), "!hit2 cow".into(), String::new(), hand("wooden_sword"), "!hit2 cow".into(), String::new()]);
    lines.extend(["kill @e[type=cow]".into(), String::new(), String::new(), "fill ~1 ~-2 ~ ~1 ~-1 ~ stone".into(), hand("air")]);
    lines.extend(["summon cow ~2 ~ ~", "!hit cow", "", "", "kill @e[type=cow]", "", "summon pig ~ ~ ~2", "!hit pig", "", "", "kill @e[type=!player]", "", ""].map(String::from));
    lines
}
