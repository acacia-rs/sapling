//! The fx2 and dig drills of bds_command_probe: darkness, fatal poison and conduit power on a
//! player, and the crack speed BDS tells of a break under haste, mining fatigue and conduit power.

pub const DIG_SHOWN: &[&str] = &["LevelEvent", "UpdateBlock", "MobEffect", "PlayerAction"];

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn wait(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

/// The actor in survival: darkness given and taken; fatal poison at 6 health, then at 1; conduit
/// power with its head under water until it would have drowned, then taken.
pub fn fx2() -> Vec<String> {
    let mut lines = owned(["kill @e[type=!player]", "", "fill ~-3 ~-1 ~-3 ~3 ~-1 ~3 stone", "fill ~-3 ~ ~-3 ~3 ~3 ~3 air", "gamemode survival @s", "", "effect @s darkness 4", ""]);
    wait(&mut lines, 5);
    lines.extend(owned(["effect @s darkness 10 1 true", "", "effect @s darkness 0", "", "damage @s 14", "", "effect @s fatal_poison 6", ""]));
    wait(&mut lines, 7);
    lines.extend(owned(["effect @s fatal_poison 4 3", ""]));
    wait(&mut lines, 5);
    lines.extend(owned(["effect @s instant_health 1 4", "", "fill ~-1 ~ ~-1 ~1 ~2 ~1 glass", "fill ~ ~ ~ ~ ~1 ~ water", "effect @s conduit_power 30", ""]));
    wait(&mut lines, 20);
    lines.extend(owned(["effect @s clear", ""]));
    wait(&mut lines, 18);
    lines.extend(owned(["fill ~-1 ~ ~-1 ~1 ~2 ~1 air", "gamemode creative @s", ""]));
    lines
}

/// The actor in survival breaks stone by hand and with a wooden pickaxe, with no effect, haste 1
/// and 3, mining fatigue 1 and 2, and conduit power 1.
pub fn dig() -> Vec<String> {
    let mut lines = owned(["kill @e[type=!player]", "fill ~-3 ~-1 ~-3 ~3 ~-1 ~3 stone", "fill ~-3 ~ ~-3 ~3 ~3 ~3 air", "gamemode survival @s", "clear @s", ""]);
    for effect in ["", "haste 30 0", "haste 30 2", "mining_fatigue 30 0", "mining_fatigue 30 1", "conduit_power 30 0"] {
        lines.push("effect @s clear".into());
        if !effect.is_empty() {
            lines.push(format!("effect @s {effect}"));
        }
        for tool in ["air", "wooden_pickaxe"] {
            lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {tool}"), "setblock ~2 ~ ~ stone".into(), String::new(), "!break 2 0 0".into(), String::new()]);
        }
    }
    lines.extend(owned(["effect @s clear", "clear @s", "gamemode creative @s", ""]));
    lines
}
