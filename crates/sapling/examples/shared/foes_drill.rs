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

/// As [`breeze`]: a vex 6 off and 1 up for thirty seconds.
pub fn vex() -> Vec<String> {
    open_floor("stone", &["summon vex ~ ~1 ~-6"], 30)
}

/// The actor in survival at midnight on the raised stone floor of [`open_floor`] (the flat
/// ground's floods sent an enderman off): an enderman is summoned, put 6 off once its first
/// teleport is done and held still, and the actor looks at a point 2.6 up it, then 0.5, 1, 1.5,
/// 2 and 3 to its side, for three seconds each, a fresh enderman each time
/// (bds-ender-findings.md: the bot did not hold `/tp facing`).
pub fn stare() -> Vec<String> {
    let mut lines = open_floor("stone", &[], 0);
    lines.truncate(lines.len() - 5);
    for side in ["0", "0.5", "1", "1.5", "2", "3"] {
        lines.extend(["summon enderman ~ ~ ~6", "", "tp @e[type=enderman] ~ ~ ~6", "effect @e[type=enderman] slowness 30 255"].map(String::from));
        lines.extend([format!("!look {side} 2.6 6"), String::new(), String::new(), String::new(), "!look 0 1.62 -10".into(), "effect @s instant_health 1 4".into(), "kill @e[type=!player]".into(), String::new()]);
    }
    lines.extend(["effect @s clear", "gamerule falldamage true", "gamemode creative @s", ""].map(String::from));
    lines
}

/// The actor in survival at midnight, breathing water, at the bottom of a pool 13 across and 5
/// deep walled in stone: a dolphin summoned in it swims ten seconds, is hit from 1.5 off, and
/// then thirty seconds pass, the actor healed each second.
pub fn dolphin() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "time set midnight", "gamemode survival @s"].map(String::from).into();
    // In fixed places: the actor is anywhere after earlier drills, and still moving after a `tp`.
    lines.extend(["fill -7 -27 -7 7 -20 7 stone", "fill -6 -26 -6 6 -20 6 water", "fill -7 -19 -7 7 -14 7 air", "effect @s water_breathing 120 0", "tp @s 0.5 -26 0.5", "", "summon dolphin 0.5 -25 3.5"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 10));
    // Held still for the hit: it swam 3 off in the second between the `tp` and the `!hit`.
    lines.extend(["effect @e[type=dolphin] slowness 10 255", "", "execute at @e[type=dolphin,c=1] run tp @s ~ ~ ~1.5 facing ~ ~0.3 ~", "!hit dolphin", "effect @e[type=dolphin] clear"].map(String::from));
    lines.extend(std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), 30));
    lines.extend(["kill @e[type=!player]", "effect @s clear", "fill -7 -27 -7 7 -20 7 stone", "tp @s 0.5 -19 0.5", "gamemode creative @s", ""].map(String::from));
    lines
}

/// On [`open_floor`], a panda of each gene that fights or flees otherwise (its spawn event sets
/// it), held still, hit from 1.5 off, then twenty seconds with the actor healed each second; the
/// aggressive one with another aggressive one 6 off, not hit, to see the anger passed on.
pub fn pandas() -> Vec<String> {
    let mut lines = open_floor("stone", &[], 0);
    lines.truncate(lines.len() - 5);
    for gene in ["aggressive", "lazy", "worried", "weak", "playful"] {
        // The bystander first: `!hit` takes the panda summoned last. BDS takes the spawn event
        // only after a rotation.
        if gene == "aggressive" {
            lines.push("summon panda ~6 ~ ~3 0 0 minecraft:panda_aggressive".into());
        }
        lines.push(format!("summon panda ~ ~ ~3 0 0 minecraft:panda_{gene}"));
        lines.extend(["effect @e[type=panda] slowness 10 255", "", "execute at @e[type=panda,c=1] run tp @s ~ ~ ~-1.5 facing ~ ~1 ~", "!hit panda", "effect @e[type=panda] clear"].map(String::from));
        lines.extend(std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), 20));
        lines.extend(["kill @e[type=!player]", "tp @s 0.5 -19 0.5", ""].map(String::from));
    }
    lines.extend(["effect @s clear", "gamerule falldamage true", "gamemode creative @s", ""].map(String::from));
    lines
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
