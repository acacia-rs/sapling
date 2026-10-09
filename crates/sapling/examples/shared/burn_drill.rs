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
