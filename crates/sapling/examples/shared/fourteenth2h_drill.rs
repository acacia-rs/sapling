//! The fourteenth wave's group H drills (research/parity/FOURTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27, the actor at 0.5 -26 0.5 facing east). `h2dmgp`: a survival actor takes `/damage` of each
//! cause as from a stand ahead and to its left, then as from the watcher and from itself. `h2dmgm`: mobs take
//! `/damage` as from the actor (survival, unhurtable): a cow by several causes, a wolf, three zombie pigmen,
//! an iron golem; a skeleton as from a zombie.

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "SetEntityData", "LevelSoundEvent", "UpdateAttributes", "EntityEvent", "SetEntityMotion", "MoveEntityDelta", "MoveEntityAbsolute", "CorrectPlayerMovePrediction", "CommandOutput", "AddEntity",
    "RemoveEntity", "MobEffect", "Animate",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "h2dmgp" => (player(), SHOWN),
        "h2dmgm" => (mobs(), SHOWN),
        _ => return None,
    })
}

const HEAL: &str = "effect @s instant_health 1 4";
const HOME: &str = "tp @s 0.5 -26 0.5 -90 0";

const CAUSES: [&str; 29] = [
    "entity_attack", "projectile", "magic", "fall", "fire", "fire_tick", "lava", "drowning", "block_explosion", "entity_explosion", "thorns", "fireworks", "lightning", "anvil", "freezing", "wither", "starve",
    "contact", "none", "void", "fly_into_wall", "magma", "stalactite", "stalagmite", "ram_attack", "sonic_boom", "campfire", "falling_block", "temperature",
];

fn player() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 6.5", "gamemode survival @s", "summon armor_stand 2.5 -26 1.5", ""]);
    for cause in CAUSES {
        lines = with(lines, &[&format!("damage @s 2 {cause} entity @e[type=armor_stand,c=1]"), "", HEAL, HOME, ""]);
    }
    for by in ["CmdWatcher", "@s"] {
        lines = with(lines, &[&format!("damage @s 2 entity_attack entity {by}"), "", HEAL, HOME, ""]);
    }
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""])
}

/// One mob of `kind` at 3.5 -26 0.5 named `name`, hurt by `damage` lines, watched `seconds` after.
fn one(mut lines: Vec<String>, summon: &str, damage: &[&str], seconds: usize) -> Vec<String> {
    lines = with(lines, &[summon, "", ""]);
    for line in damage {
        lines = with(lines, &[line, "", ""]);
    }
    waits(&mut lines, seconds);
    with(lines, &["kill @e[type=!player]", HOME, ""])
}

fn mobs() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 6.5", "gamemode survival @s", "effect @s resistance 1000 255 true", ""]);
    lines = one(lines, "summon cow c 3.5 -26 0.5", &["damage @e[name=c] 1 entity_attack entity @s", "damage @e[name=c] 1 magic entity @s", "damage @e[name=c] 1 fall entity @s"], 3);
    lines = one(lines, "summon cow c 3.5 -26 0.5", &["damage @e[name=c] 1 entity_attack"], 3);
    lines = one(lines, "summon wolf w 3.5 -26 0.5", &["damage @e[name=w] 1 entity_attack entity @s"], 6);
    lines = with(lines, &["summon zombie_pigman q -4.5 -26 -4.5", "summon zombie_pigman r -4.5 -26 4.5"]);
    lines = one(lines, "summon zombie_pigman p 3.5 -26 0.5", &["damage @e[name=p] 1 entity_attack entity @s"], 6);
    lines = one(lines, "summon iron_golem g 3.5 -26 0.5", &["damage @e[name=g] 1 entity_attack entity @s"], 6);
    lines = with(lines, &["gamemode creative @s", "summon zombie z -4.5 -26 0.5", "effect @e[name=z] slowness 1000 255 true"]);
    lines = one(lines, "summon skeleton s 3.5 -26 0.5", &["damage @e[name=s] 1 entity_attack entity @e[name=z]"], 6);
    with(lines, &["effect @s clear", ""])
}
