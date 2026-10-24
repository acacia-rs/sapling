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
