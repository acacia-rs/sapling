//! The thirteenth wave's group G drills (research/parity/THIRTEENTH-BRIEF.md). `gxbarter`: a piglin
//! handed an ingot (its `admire` again as its hand empties), then one hit and handed ingots 2, 12
//! and 22 s on (`cooldown_after_being_attacked` 20). `gxfellows`: a piglin in sight of the survival
//! actor and another shut in stone 12 off, then the first hit (`broadcast_anger`). `gxcreak`: a
//! creaking frozen by the survival actor's look, the actor turned creative, survival, looking away,
//! creative again (`deactivate`). `gxspit`: a llama and a panda hit (the llama's charge, the panda's
//! stand before it walks up).

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelSoundEvent", "MobEquipment", "UpdateAttributes", "AddItemEntity", "TakeItemEntity",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "gxbarter" => (barter(), SHOWN),
        "gxfellows" => (fellows(), SHOWN),
        "gxcreak" => (creak(), SHOWN),
        "gxspit" => (spit(), SHOWN),
        "gxwither" => (wither(), BUILT),
        _ => return None,
    })
}

const BUILT: &[&str] = &["AddEntity", "UpdateBlock", "LevelEvent", "LevelEventGeneric", "LevelSoundEvent", "SpawnParticleEffect"];

/// An upright wither T, its last skull put by the actor (twelfth8's `g8wither`), its building's particles shown.
fn wither() -> Vec<String> {
    let skull = "wither_skeleton_skull [\"facing_direction\"=1]";
    let mut lines = with(floor(), &["replaceitem entity @s slot.weapon.mainhand 0 wither_skeleton_skull 64", "setblock 6 -24 1 stone", "tp @s 6.5 -23 1.5 90 30"]);
    lines.extend(["setblock 3 -26 0 soul_sand", "fill 3 -25 -1 3 -25 1 soul_sand"].map(String::from));
    lines.extend([format!("setblock 3 -24 -1 {skull}"), format!("setblock 3 -24 0 {skull}"), "!place -3 -2 0".into(), String::new(), String::new()]);
    with(lines, &["kill @e[type=wither]", "fill 1 -26 -3 7 -23 3 air", "clear @s", ""])
}

fn held(kind: &str) -> String {
    format!("effect @e[type={kind}] slowness 120 255 true")
}

fn aside() -> Vec<String> {
    with(floor(), &["tp CmdWatcher 14.5 -26 7.5", "time set noon"])
}

fn barter() -> Vec<String> {
    let mut lines = with(aside(), &["replaceitem entity @s slot.weapon.mainhand 0 gold_ingot 8", "summon piglin 2.0 -26 0.5", &held("piglin"), "", "!interact piglin"]);
    waits(&mut lines, 10);
    lines.extend(["kill @e[type=piglin]", "", "summon piglin 2.0 -26 0.5", &held("piglin"), "", "!hit piglin", "tp @e[type=piglin] 2.0 -26 0.5"].map(String::from));
    for _ in 0..3 {
        waits(&mut lines, 1);
        lines.extend(["tp @e[type=piglin] 2.0 -26 0.5".into(), "!interact piglin".into()]);
        waits(&mut lines, 8);
    }
    waits(&mut lines, 10);
    with(lines, &["kill @e[type=!player]", "replaceitem entity @s slot.weapon.mainhand 0 air"])
}

fn fellows() -> Vec<String> {
    let mut lines = with(aside(), &["difficulty normal", "fill 1 -26 10 5 -23 14 stone", "fill 3 -26 12 3 -25 12 air", "effect @s resistance 120 255 true"]);
    lines.extend(["summon piglin 3.5 -26 12.5 0 0 minecraft:spawn_adult", "summon piglin 3.5 -26 0.5 0 0 minecraft:spawn_adult", &held("piglin"), "gamemode survival @s"].map(String::from));
    waits(&mut lines, 4);
    lines.extend(["!hit piglin".into()]);
    waits(&mut lines, 5);
    with(lines, &["gamemode creative @s", "kill @e[type=!player]", "fill 1 -26 10 5 -23 14 air", "difficulty easy"])
}

fn creak() -> Vec<String> {
    let mut lines = with(aside(), &["time set midnight", "summon creaking 0.5 -26 -6.5", &held("creaking"), "tp @s 0.5 -26 0.5 facing 0.5 -25 -6.5", "effect @s resistance 120 255 true", "gamemode survival @s"]);
    waits(&mut lines, 4);
    lines.push("gamemode creative @s".into());
    waits(&mut lines, 3);
    lines.push("gamemode survival @s".into());
    waits(&mut lines, 3);
    lines.push("tp @s 0.5 -26 0.5 facing 0.5 -25 8".into());
    waits(&mut lines, 3);
    lines.push("gamemode creative @s".into());
    waits(&mut lines, 3);
    with(lines, &["kill @e[type=!player]"])
}

fn spit() -> Vec<String> {
    let mut lines = with(aside(), &["summon llama 3.5 -26 0.5", "", "!hit llama"]);
    waits(&mut lines, 6);
    lines.extend(["kill @e[type=!player]", "", "summon panda 3.5 -26 0.5", "", "!hit panda"].map(String::from));
    waits(&mut lines, 10);
    with(lines, &["kill @e[type=!player]"])
}
