//! Drills of blasts: what they break and leave, and how they hurt and throw mobs behind cover.

/// Packets shown of these drills.
pub const BLAST_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "AddItemEntity", "AddEntity", "RemoveEntity", "SetEntityMotion", "EntityEvent", "LevelEvent"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "blasts" => blasts(),
            "creeperdrops" => creeper_drops(),
            _ => return None,
        },
        BLAST_SHOWN,
    ))
}

/// A dirt floor over stone east of the actor, which stands at the middle of its cell.
fn floor() -> Vec<String> {
    ["kill @e[type=!player]", "gamerule domobspawning false", "gamemode creative @s", "tp @s 0.5 ~ 0.5 -90 0", "fill ~-8 ~-2 ~-8 ~12 ~-2 ~8 stone", "fill ~-8 ~-1 ~-8 ~12 ~-1 ~8 dirt", "fill ~-8 ~ ~-8 ~12 ~6 ~8 air", ""]
        .map(String::from)
        .into()
}

fn waits(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

/// TNT five blocks east with `tntexplosiondropdecay` on, pigs that take no harm (resistance 5)
/// three blocks from it: east in the open, north behind a stone wall one high, south behind an
/// obsidian wall three high. Then the same with the rule off and the pigs five blocks off.
pub fn blasts() -> Vec<String> {
    let mut lines = floor();
    for (rule, far) in [("true", 3), ("false", 5)] {
        lines.extend([
            format!("gamerule tntexplosiondropdecay {rule}"),
            "fill ~4 ~ ~-2 ~6 ~ ~-2 stone".into(),
            "fill ~4 ~ ~2 ~6 ~2 ~2 obsidian".into(),
            format!("summon pig ~{} ~ ~", 5 + far),
            format!("summon pig ~5 ~ ~-{far}"),
            format!("summon pig ~5 ~ ~{far}"),
            "effect @e[type=pig] slowness 100 255 true".into(),
            "effect @e[type=pig] resistance 100 4 true".into(),
            String::new(),
            "summon tnt ~5 ~ ~".into(),
        ]);
        waits(&mut lines, 6);
        lines.extend(["kill @e[type=!player]", "fill ~-8 ~-1 ~-8 ~12 ~-1 ~8 dirt", "fill ~-8 ~ ~-8 ~12 ~6 ~8 air", ""].map(String::from));
    }
    lines.push("gamerule tntexplosiondropdecay false".into());
    lines
}

/// A creeper bursting by the actor, which stands in survival and takes no harm, on the dirt
/// floor, three times: what the dirt it broke leaves.
pub fn creeper_drops() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["difficulty easy", "gamerule mobgriefing true", "effect @s resistance 100 4 true", "gamemode survival @s"].map(String::from));
    for _ in 0..3 {
        lines.extend(["summon creeper ~6 ~ ~".into()]);
        waits(&mut lines, 8);
        lines.extend(["kill @e[type=!player]", "fill ~-8 ~-1 ~-8 ~12 ~-1 ~8 dirt", "fill ~-8 ~ ~-8 ~12 ~6 ~8 air", "tp @s 0.5 ~ 0.5 -90 0", ""].map(String::from));
    }
    lines.extend(["gamemode creative @s", "effect @s clear", ""].map(String::from));
    lines
}
