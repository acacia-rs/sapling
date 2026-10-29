//! The fights and hits2 drills of bds_command_probe (docs/research/mob-attacks.md, mob-damage.md
//! "Hits"), on harm_drill.rs's floor; the actor in survival, healed every second.

use crate::harm::{start, summon, wait};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "fights" => fights(),
            "hits2" => hits2(),
            "kinds3" => kinds3(),
            _ => return None,
        },
        crate::harm::SHOWN,
    ))
}

fn healed(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), seconds));
}

fn held(lines: &mut Vec<String>) {
    lines.push("effect @e[type=!player] slowness 120 255 true".into());
}

/// A zombie beside the actor on normal and hard, a cave spider on both, a baby zombie 10 off, a
/// spider 7 off at noon then midnight, a zombie hit twice; creepers held still whose prey goes 8
/// off, 5 off and into creative; a forced blast behind an obsidian wall and one in the open over
/// dirt and sand; a skeleton shooting past a cow; a zombie behind glass, behind leaves, and one
/// walled off after it took the actor.
pub fn fights() -> Vec<String> {
    let mut lines = start("midnight", 0);
    lines.extend(["gamemode survival @s", ""].map(String::from));
    let clear = |lines: &mut Vec<String>| lines.extend(["kill @e[type=!player]", "effect @s clear", "effect @s instant_health 1 4", ""].map(String::from));
    for difficulty in ["normal", "hard"] {
        lines.extend([format!("difficulty {difficulty}"), "summon zombie 0.5 -26 -1.5".into()]);
        healed(&mut lines, 5);
        clear(&mut lines);
    }
    for difficulty in ["normal", "hard"] {
        lines.extend([format!("difficulty {difficulty}"), "summon cave_spider 0.5 -26 -2.5".into()]);
        healed(&mut lines, 8);
        clear(&mut lines);
    }
    lines.extend(["difficulty easy", "summon zombie 0.5 -26 -9.5 0 0 minecraft:as_baby"].map(String::from));
    healed(&mut lines, 6);
    clear(&mut lines);
    lines.extend(["time set noon", "summon spider 0.5 -26 -6.5"].map(String::from));
    healed(&mut lines, 5);
    lines.push("time set midnight".into());
    healed(&mut lines, 5);
    clear(&mut lines);
    lines.extend(["summon zombie 0.5 -26 -2.5", "", "!hit zombie"].map(String::from));
    healed(&mut lines, 2);
    lines.push("!hit zombie".into());
    healed(&mut lines, 3);
    clear(&mut lines);
    lines.push("effect @s resistance 120 4 true".into());
    for away in ["tp @s 0.5 -26 6.5", "tp @s 0.5 -26 3.5", "gamemode creative @s"] {
        // It is lit some 0.6 s after it comes and bursts 1.5 s after that (bds-fights.txt): the prey leaves at 1 s.
        lines.extend(["tp @s 0.5 -26 0.5", "summon creeper 0.5 -26 -2", away].map(String::from));
        held(&mut lines);
        healed(&mut lines, 4);
        lines.extend(["gamemode survival @s", "kill @e[type=!player]", "fill -20 -27 -20 20 -27 20 stone", "fill -20 -26 -20 20 -16 20 air", ""].map(String::from));
    }
    lines.extend(["tp @s 0.5 -26 0.5", "effect @s clear", "effect @s health_boost 120 4 true", "effect @s instant_health 1 9 true", "fill -2 -26 -2 3 -24 -2 obsidian"].map(String::from));
    lines.push("summon creeper 0.5 -26 -4.5 0 0 minecraft:start_exploding_forced".into());
    held(&mut lines);
    wait(&mut lines, 4);
    lines.extend(["effect @s instant_health 1 9 true", "fill -2 -26 -2 3 -24 -2 air", "fill -20 -27 -20 20 -27 20 stone", "fill -2 -27 -7 0 -27 -3 dirt", "fill 1 -27 -7 3 -27 -3 sand"].map(String::from));
    lines.push("summon creeper 0.5 -26 -4.5 0 0 minecraft:start_exploding_forced".into());
    held(&mut lines);
    wait(&mut lines, 4);
    lines.extend(["kill @e[type=!player]", "fill -20 -27 -20 20 -27 20 stone", "fill -20 -26 -20 20 -16 20 air", "effect @s instant_health 1 9 true", ""].map(String::from));
    lines.extend(["summon cow 0.5 -26 -4.5", "summon skeleton 0.5 -26 -9.5"].map(String::from));
    held(&mut lines);
    healed(&mut lines, 10);
    clear(&mut lines);
    for wall in ["glass", "oak_leaves [\"persistent_bit\"=true]"] {
        lines.extend([format!("fill -10 -26 -4 10 -24 -4 {wall}"), "summon zombie 0.5 -26 -8.5".into()]);
        healed(&mut lines, 5);
        clear(&mut lines);
    }
    lines.extend(["fill -10 -26 -4 10 -24 -4 air", "summon zombie 0.5 -26 -8.5"].map(String::from));
    healed(&mut lines, 2);
    lines.push("fill -10 -26 -4 10 -24 -4 stone".into());
    healed(&mut lines, 6);
    clear(&mut lines);
    lines.extend(["gamemode creative @s", "fill -10 -26 -4 10 -24 -4 air", ""].map(String::from));
    lines
}

/// One mob held still 2 north of the actor for each weapon: `kind` summoned, `item` given with
/// `enchants`, hit `times` a second apart, watched `after` seconds.
fn struck(lines: &mut Vec<String>, kind: &str, item: &str, enchants: &[&str], times: usize, after: usize) {
    // Mobs push the actor off its mark and `!hit` reaches only 3 (bds-hits2.txt).
    lines.extend(["kill @e[type=!player]".into(), "tp @s 0.5 -26 0.5".into(), format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")]);
    lines.extend(enchants.iter().map(|enchant| format!("enchant @s {enchant}")));
    lines.push(summon(kind, 0.5, -26.0, -1.5));
    held(lines);
    // A cow with 30 health outlasts the sharper swords.
    if kind == "cow" {
        lines.extend(["effect @e[type=cow] health_boost 120 4 true", "effect @e[type=cow] instant_health 1 4 true"].map(String::from));
    }
    lines.push(String::new());
    let name = kind.split(' ').next().unwrap_or(kind);
    for _ in 0..times {
        lines.extend([format!("!hit {name}"), String::new()]);
    }
    wait(lines, after);
}

/// Sharpness, smite and bane against a plain sword, a zombie in a diamond chestplate, fire aspect
/// and knockback, a wooden sword worn by three hits, crits while jumping, and the experience of a
/// grown cow, a baby cow and a baby zombie killed.
pub fn hits2() -> Vec<String> {
    let mut lines = start("midnight", 0);
    lines.extend(["gamemode survival @s", "effect @s resistance 300 4 true", ""].map(String::from));
    struck(&mut lines, "cow", "diamond_sword", &[], 1, 1);
    struck(&mut lines, "cow", "diamond_sword", &["sharpness 1"], 1, 1);
    struck(&mut lines, "cow", "diamond_sword", &["sharpness 5"], 1, 1);
    struck(&mut lines, "zombie", "diamond_sword", &[], 1, 1);
    struck(&mut lines, "zombie", "diamond_sword", &["smite 2"], 1, 1);
    struck(&mut lines, "spider", "diamond_sword", &[], 1, 1);
    struck(&mut lines, "spider", "diamond_sword", &["bane_of_arthropods 2"], 1, 1);
    lines.extend(["kill @e[type=!player]", "tp @s 0.5 -26 0.5", "summon zombie 0.5 -26 -1.5", "replaceitem entity @e[type=zombie] slot.armor.chest 0 diamond_chestplate"].map(String::from));
    held(&mut lines);
    lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 diamond_sword", "", "!hit zombie", "", ""].map(String::from));
    struck(&mut lines, "cow", "iron_sword", &["fire_aspect 1"], 1, 10);
    struck(&mut lines, "cow", "iron_sword", &["knockback 2"], 1, 2);
    struck(&mut lines, "cow", "wooden_sword", &[], 3, 1);
    lines.push("!jump".into());
    struck(&mut lines, "cow", "diamond_sword", &[], 4, 1);
    lines.extend(["!stop", ""].map(String::from));
    for kind in ["cow", "cow 0 0 minecraft:entity_born", "zombie 0 0 minecraft:as_baby"] {
        lines.extend(["kill @e[type=!player]", "tp @s 0.5 -26 0.5", "replaceitem entity @s slot.weapon.mainhand 0 diamond_sword"].map(String::from));
        lines.extend([summon(kind, 0.5, -26.0, -1.5), String::new()]);
        held(&mut lines);
        let name = kind.split(' ').next().unwrap_or(kind);
        lines.extend([format!("!hit {name}"), String::new(), format!("!hit {name}"), String::new(), format!("!hit {name}")]);
        wait(&mut lines, 3);
    }
    lines.extend(["gamemode creative @s", "effect @s clear", "clear @s", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// On a stone floor on the flat ground (y -61, where teleports land): a witch held 6 off the actor
/// with the watcher 2 off it, both in survival, and two cows held beside it (splash times by
/// distance); an enderman stared at by the actor in a carved pumpkin, then bare-headed; three
/// endermen left in the rain.
pub fn kinds3() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "weather clear", "time set midnight", "effect @s clear", "clear @s"].map(String::from).into();
    lines.extend(["fill -20 -61 -20 20 -61 20 stone", "fill -20 -60 -20 20 -50 20 air", "tp @s 0.5 -60 0.5", "tp CmdWatcher 2.5 -60 0.5", "gamemode survival @s", "gamemode survival CmdWatcher", ""].map(String::from));
    lines.extend(["summon cow -1.5 -60 0.5", "summon cow 0.5 -60 2.5", "summon witch 0.5 -60 -5.5"].map(String::from));
    held(&mut lines);
    for _ in 0..20 {
        lines.extend(["effect @s instant_health 1 4", "effect CmdWatcher instant_health 1 4"].map(String::from));
    }
    lines.extend(["kill @e[type=!player]", "effect @a clear", "gamemode creative CmdWatcher", "tp CmdWatcher 18.5 -60 18.5", ""].map(String::from));
    for head in ["carved_pumpkin", "air"] {
        lines.extend([format!("replaceitem entity @s slot.armor.head 0 {head}"), "summon enderman 0.5 -60 6.5".into(), String::new(), "tp @e[type=enderman] 0.5 -60 6.5".into()]);
        held(&mut lines);
        lines.extend(["!look 0 2.6 6", "", "", "", "!look 0 1.62 -10"].map(String::from));
        healed(&mut lines, 3);
        lines.extend(["kill @e[type=!player]", "tp @s 0.5 -60 0.5", ""].map(String::from));
    }
    lines.extend(["gamemode creative @s", "effect @s clear", "weather rain", "summon enderman -6.5 -60 -6.5", "summon enderman 6.5 -60 -6.5", "summon enderman 0.5 -60 8.5"].map(String::from));
    wait(&mut lines, 15);
    lines.extend(["weather clear", "kill @e[type=!player]", ""].map(String::from));
    lines
}
