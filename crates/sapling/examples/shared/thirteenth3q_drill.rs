//! The thirteenth wave's group Q drills (research/parity/THIRTEENTH-BRIEF.md), on warden_drill.rs's floor
//! (stone at y -27, the warded survival actor at the middle). `q3evoker`: eight held evokers 9 off, their
//! vexes killed each second (which spell each cast is). `q3creeper`: a creeper's fuse lit at 2 off, the
//! actor then put 5.5, 6.5 and 7.5 off. `q3baby`: a baby zombie 7 off on easy, three times. `q3drowned`:
//! three trident drowned 6, 8 and 10 off in a pool (swing timing). `q3arrows`: a skeleton's dodged arrows
//! stood on and left a minute. `q3witch`: a witch throwing at the actor 6 off, then
//! one 14 off drinking as it walks.

use super::creatures::{waits, with};
use super::warden::{healed, start};

#[path = "thirteenth3q_more.rs"]
mod more;

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "TakeItemEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "MobEffect", "MobEquipment", "UpdateAttributes", "CommandOutput", "Animate", "SetEntityLink", "MoveEntityAbsolute",
];

const BLOCKS: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "LevelSoundEvent", "LevelEvent", "AddItemEntity", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "q3evoker" => (evoker(), SHOWN),
        "q3creeper" => (creeper(), SHOWN),
        "q3baby" => (baby(), SHOWN),
        "q3drowned" => (drowned(), SHOWN),
        "q3arrows" => (arrows(), SHOWN),
        "q3witch" => (witch(), SHOWN),
        "q3keep" => (more::keep(), SHOWN),
        "q3kept" => (more::kept(), SHOWN),
        "q3crowd" => (more::crowd(), SHOWN),
        "q3wind" => (more::wind(), SHOWN),
        "q3love" => (more::love(), SHOWN),
        "q3integ" => (more::integrity(), BLOCKS),
        "q3stare" => (more::stare(), SHOWN),
        "q3integ2" =>(more::integrity_twice(), BLOCKS),
        "q3brush" =>(more::brush(), BLOCKS),
        "q3seats" => (more::seats(), SHOWN),
        "q3despawn" => (more::despawn(), &["CommandOutput", "RemoveEntity"]),
        _ => return None,
    })
}

fn warded(time: &str, more: &[&str]) -> Vec<String> {
    with(start(time), &[&["effect @s resistance 900 4 true"], more].concat())
}

fn evoker() -> Vec<String> {
    let mut lines = warded("noon", &["difficulty normal"]);
    for (x, z) in [(9, 0), (-9, 0), (0, 9), (0, -9), (6, 6), (-6, 6), (6, -6), (-6, -6)] {
        lines.push(format!("summon evocation_illager {}.5 -26 {}.5", x, z));
    }
    lines.push("effect @e[type=evocation_illager] slowness 120 255 true".into());
    for _ in 0..50 {
        lines.extend(["kill @e[type=vex]".into(), "effect @s instant_health 1 4".into()]);
    }
    with(lines, &["kill @e[type=!player]", "gamemode creative @s"])
}

fn creeper() -> Vec<String> {
    let mut lines = warded("midnight", &["gamerule mobgriefing false"]);
    for gap in ["4", "5", "6"] {
        // The fuse (30 ticks) is lit some 0.5 s after the summon, so the step away comes a second in.
        lines.extend(["tp @s 0.5 -26 0.5 180 0", "summon creeper 0.5 -26 -1.5"].map(String::from));
        lines.push(format!("tp @s 0.5 -26 {gap} 180 0"));
        healed(&mut lines, 4);
        lines.push("kill @e[type=creeper]".into());
    }
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "gamerule mobgriefing true"])
}

fn baby() -> Vec<String> {
    let mut lines = warded("midnight", &[]);
    for _ in 0..3 {
        lines.extend(["tp @s 0.5 -26 0.5 180 0", "summon zombie 0.5 -26 -6.5 0 0 minecraft:as_baby"].map(String::from));
        healed(&mut lines, 8);
        lines.push("kill @e[type=zombie]".into());
    }
    with(lines, &["kill @e[type=!player]", "gamemode creative @s"])
}

fn drowned() -> Vec<String> {
    let mut lines = warded("midnight", &["fill 3 -27 -8 14 -21 8 water", "tp @s 0.5 -26 0.5 -90 0"]);
    lines.extend(["summon drowned 6.5 -25 -4.5 0 0 minecraft:as_ranged_adult", "summon drowned 8.5 -25 0.5 0 0 minecraft:as_ranged_adult", "summon drowned 10.5 -25 4.5 0 0 minecraft:as_ranged_adult"].map(String::from));
    healed(&mut lines, 25);
    with(lines, &["kill @e[type=!player]", "fill 3 -27 -8 14 -21 8 air", "fill 3 -27 -8 14 -27 8 stone", "gamemode creative @s"])
}

/// The actor dodges a held skeleton's arrows, stepping 9 aside each second, then stands on the nearest
/// lying arrow each second, then stands off a minute.
fn arrows() -> Vec<String> {
    let mut lines = warded("midnight", &["summon skeleton 0.5 -26 -9.5", "effect @e[type=skeleton] slowness 60 255 true"]);
    for x in [-4, 5].repeat(8) {
        lines.push(format!("tp @s {x}.5 -26 0.5 180 0"));
    }
    lines.extend(["kill @e[type=skeleton]", "testfor @e[type=arrow]"].map(String::from));
    lines.extend(std::iter::repeat_n("tp @s @e[type=arrow,c=1]".to_owned(), 6));
    lines.extend(["testfor @e[type=arrow]", "tp @s 0.5 -26 -15.5 180 0"].map(String::from));
    waits(&mut lines, 66);
    lines.push("testfor @e[type=arrow]".into());
    with(lines, &["kill @e[type=!player]", "gamemode creative @s"])
}

/// The first witch is held; the second walks at the actor 13 off (swiftness drunk on the way).
fn witch() -> Vec<String> {
    let mut lines = warded("midnight", &["tp @s 0.5 -26 0.5 180 0", "summon witch 0.5 -26 -5.5", "effect @e[type=witch] slowness 60 255 true"]);
    healed(&mut lines, 20);
    lines.extend(["kill @e[type=witch]", "effect @s clear", "effect @s resistance 900 4 true", "tp @s 0.5 -26 6.5 180 0", "summon witch 0.5 -26 -6.5"].map(String::from));
    healed(&mut lines, 20);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s"])
}
