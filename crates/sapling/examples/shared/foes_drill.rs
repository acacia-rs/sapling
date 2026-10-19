//! The witch, ender, goat, evoker and shulker drills of bds_command_probe: mobs that throw, cast,
//! teleport or ram (docs/research/mob-attacks.md).

/// A stone floor 16 by 53 at midnight on easy, the actor in survival on it, 20 in from one end.
const LANE: [&str; 9] = ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "time set midnight", "fill ~-3 ~-1 ~-12 ~12 ~-1 ~40 stone", "fill ~-3 ~ ~-12 ~12 ~6 ~40 air", "gamemode survival @s", "tp @s ~6 ~ ~20", ""];

/// On the [`LANE`], each of `mobs` in turn: its lines run, then the actor healed each second for
/// so many seconds, then it is killed.
fn faced(mobs: &[(&[&str], usize)]) -> Vec<String> {
    let mut lines: Vec<String> = LANE.map(String::from).into();
    for (summon, seconds) in mobs {
        lines.extend(summon.iter().map(|line| line.to_string()));
        lines.extend(std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), *seconds));
        lines.extend(["kill @e[type=!player]", "effect @s clear", "", ""].map(String::from));
    }
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}

/// A witch 6 and then 12 blocks off for twenty seconds each.
pub fn witch() -> Vec<String> {
    faced(&[(&["summon witch ~ ~ ~-6"], 20), (&["summon witch ~ ~ ~-12"], 20)])
}

/// An evoker 10 off for thirty seconds, then one held still 2 off for fifteen.
pub fn evoker() -> Vec<String> {
    faced(&[(&["summon evocation_illager ~ ~ ~-10"], 30), (&["summon evocation_illager ~ ~ ~-2", "effect @e[type=evocation_illager] slowness 30 255"], 15)])
}

/// A shulker 6 and then 12 blocks off for twenty seconds each.
pub fn shulker() -> Vec<String> {
    faced(&[(&["summon shulker ~ ~ ~-6"], 20), (&["summon shulker ~ ~ ~-12"], 20)])
}

/// The actor in survival at midnight on an obsidian floor 41 square, which a fireball cannot
/// break, with fall damage off: a ghast 12 off and 6 up, held still so that it stays in range,
/// for forty seconds, healed each second (bds-ghast-findings.md: one let loose wandered off).
pub fn ghast() -> Vec<String> {
    open_floor("obsidian", &["summon ghast ~ ~6 ~-12", "effect @e[type=ghast] slowness 60 255"], 40)
}

/// As [`ghast`] on stone: a breeze 8 off for thirty seconds.
pub fn breeze() -> Vec<String> {
    open_floor("stone", &["summon breeze ~ ~ ~-8"], 30)
}

/// A floor of `block` 41 square at y -20 with 20 of air over it, the actor in survival on it at
/// midnight with fall damage off; `summon` run, then the actor healed each second for `seconds`.
fn open_floor(block: &str, summon: &[&str], seconds: usize) -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamerule domobspawning false", "gamerule falldamage false", "difficulty easy", "time set midnight", "gamemode survival @s"].map(String::from).into();
    // Split under fill's limit of 32768 blocks.
    lines.extend([format!("fill ~-20 ~-1 ~-20 ~20 ~-1 ~20 {block}"), "fill ~-20 ~ ~-20 ~20 ~9 ~20 air".into(), "fill ~-20 ~10 ~-20 ~20 ~20 ~20 air".into(), String::new()]);
    lines.extend(summon.iter().map(|line| line.to_string()));
    lines.extend(std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), seconds));
    lines.extend(["kill @e[type=!player]", "effect @s clear", "gamerule falldamage true", "gamemode creative @s", ""].map(String::from));
    lines
}

/// The actor in survival at midnight on a dry stone floor 41 blocks square on the flat ground,
/// where an enderman's teleports land: one held still, hit from 1.5 off and let go, then thirty
/// seconds healed every second (research/captures/bds-ender-findings.md has an earlier run's stares).
pub fn ender() -> Vec<String> {
    let mut lines: Vec<String> =
        ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "time set midnight", "gamemode survival @s", "tp @s ~ -60 ~", "fill ~-20 ~-1 ~-20 ~20 ~-1 ~20 stone", "fill ~-20 ~ ~-20 ~20 ~5 ~20 air", ""].map(String::from).into();
    lines.extend(["summon enderman ~ ~ ~2", "effect @e[type=enderman] slowness 10 255", "", "execute at @e[type=enderman,c=1] run tp @p ~ ~ ~1.5 facing ~ ~1.5 ~", "!hit enderman", "effect @e[type=enderman] clear"].map(String::from));
    lines.extend(std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), 30));
    lines.extend(["kill @e[type=!player]", "effect @s clear", "gamemode creative @s", ""].map(String::from));
    lines
}

/// Eight goats 5 off in a ring about the actor, penned with it in glass 13 across on a stone floor
/// above the flat ground, for six minutes, past the longest a first ram waits; the actor healed
/// every second (research/captures/bds-goat-findings.md: one goat rammed nobody in five).
pub fn goat() -> Vec<String> {
    let mut lines: Vec<String> =
        ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "time set noon", "gamemode survival @s", "fill ~-8 ~-1 ~-8 ~8 ~-1 ~8 stone", "fill ~-8 ~ ~-8 ~8 ~5 ~8 air", "fill ~-7 ~ ~-7 ~7 ~2 ~7 glass", "fill ~-6 ~ ~-6 ~6 ~2 ~6 air", ""].map(String::from).into();
    for (x, z) in [(5, 0), (-5, 0), (0, 5), (0, -5), (4, 4), (-4, 4), (4, -4), (-4, -4)] {
        lines.push(format!("summon goat ~{x} ~ ~{z}"));
    }
    lines.extend(std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), 360));
    lines.extend(["kill @e[type=!player]", "effect @s clear", "gamemode creative @s", ""].map(String::from));
    lines
}
