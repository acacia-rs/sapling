//! The thirteenth wave's group AH drills (research/parity/THIRTEENTH-BRIEF.md), on warden_drill.rs's floor
//! (stone at y -27, the actor at 0.5 -26 0.5, mob loot off so nothing dropped pushes). `ah3crowd`: cows
//! summoned beside the still actor at box gaps 0 to 0.13, then beside a cow at gaps 0.05 to 0.25 (the
//! summon tick's push shows in AddEntity's velocity); `ah3crowd2` the actor's gaps 0.15 to 0.24.
//! `ah3slime`: medium slimes (`spawn_medium`; BDS runs no `minecraft:`-prefixed one) at gaps 0.08 to 0.25
//! from the actor, then one beside it under resistance 5. `ah3lag`/`ah3lag2`: a zombie in a glass or
//! iron bars cell with the actor stepped round it (head and body yaws). `ah3stop`/`ah3stop2`: husks
//! walking up to the still actor, the second with glass behind it (where they stop). `ah3spider`: a spider by night, kept into noon, and one summoned at noon. `ah3nugget`: gold
//! nuggets dropped before an armed and a bare piglin. `ah3tempt`: a cow 5, 9, 11 and 14 off the actor
//! holding wheat.

use super::creatures::{waits, with};
use super::warden::start;

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "TakeItemEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "EntityEvent", "MobEffect", "MobEquipment", "MobArmorEquipment", "UpdateAttributes", "CommandOutput", "Animate", "MoveEntityAbsolute",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ah3crowd" => (crowd(), SHOWN),
        "ah3crowd2" => (crowd_wide(), SHOWN),
        "ah3slime" => (slime(), SHOWN),
        "ah3lag" => (lag("glass"), SHOWN),
        "ah3lag2" => (lag("iron_bars"), SHOWN),
        "ah3stop" => (stop(false), SHOWN),
        "ah3stop2" => (stop(true), SHOWN),
        "ah3spider" => (spider(), SHOWN),
        "ah3nugget" => (nugget(), SHOWN),
        "ah3tempt" => (tempt(), SHOWN),
        _ => return None,
    })
}

fn quiet(time: &str) -> Vec<String> {
    with(start(time), &["gamerule domobloot false", "effect @s resistance 900 4 true"])
}

/// Wider player gaps than `crowd`'s, each twice.
fn crowd_wide() -> Vec<String> {
    let mut lines = quiet("noon");
    for gap in ["0.15", "0.17", "0.19", "0.21", "0.24", "0.15", "0.17", "0.19", "0.21", "0.24"] {
        let x: f32 = 1.25 + gap.parse::<f32>().unwrap();
        lines.extend([format!("summon cow {x} -26 0.5 0 0"), String::new(), String::new(), "kill @e[type=cow]".into()]);
    }
    with(lines, &["gamerule domobloot true"])
}

/// Player half width 0.3 and cow 0.45: a cow at x 1.25 + gap; then cow pairs 0.9 + gap apart at z 8.5.
fn crowd() -> Vec<String> {
    let mut lines = quiet("noon");
    for gap in ["0", "0.02", "0.04", "0.06", "0.08", "0.1", "0.13"] {
        let x: f32 = 1.25 + gap.parse::<f32>().unwrap();
        lines.extend([format!("summon cow {x} -26 0.5 0 0"), String::new(), String::new(), "kill @e[type=cow]".into()]);
    }
    for gap in ["0.05", "0.1", "0.15", "0.19", "0.25"] {
        let x: f32 = 10.4 + gap.parse::<f32>().unwrap();
        lines.extend(["summon cow 9.5 -26 8.5 0 0".into(), format!("summon cow {x} -26 8.5 0 0"), String::new(), String::new(), "kill @e[type=cow]".into()]);
    }
    with(lines, &["gamerule domobloot true"])
}

/// Medium slime half width 0.52: at x 0.82 + gap. Survival on easy, health boosted.
fn slime() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamerule domobloot false", "effect @s regeneration 900 4 true"]);
    for gap in ["0.08", "0.14", "0.17", "0.25"] {
        let x: f32 = 1.32 + gap.parse::<f32>().unwrap();
        lines.extend([format!("summon slime {x} -26 0.5 90 0 spawn_medium")]);
        waits(&mut lines, 3);
        lines.extend(["kill @e[type=slime]".into(), "kill @e[type=slime]".into()]);
    }
    lines.extend(["effect @s resistance 30 4 true", "summon slime 1.4 -26 0.5 90 0 spawn_medium"].map(String::from));
    waits(&mut lines, 3);
    with(lines, &["kill @e[type=slime]", "kill @e[type=slime]", "gamerule domobloot true"])
}

/// A zombie in a one-cell `wall` box at 6.5 -26 6.5, the actor stepped 5 off it north, east, south, west.
fn lag(wall: &str) -> Vec<String> {
    let mut lines = with(quiet("midnight"), &[&format!("fill 5 -26 5 7 -23 7 {wall}"), "fill 6 -26 6 6 -24 6 air", "summon zombie 6.5 -26 6.5 0 0"]);
    for at in ["6.5 -26 1.5", "11.5 -26 6.5", "6.5 -26 11.5", "1.5 -26 6.5", "6.5 -26 1.5"] {
        lines.push(format!("tp @s {at}"));
        waits(&mut lines, 2);
    }
    with(lines, &["kill @e[type=!player]", "fill 5 -26 5 7 -23 7 air", "tp @s 0.5 -26 0.5"])
}

/// A husk 6 off in x, then one 6 off in z, walking up to the still actor (with a glass wall behind it
/// in `walled`, so its hits' throws do not move it).
fn stop(walled: bool) -> Vec<String> {
    let mut lines = quiet("noon");
    if walled {
        lines.extend(["fill -1 -26 -3 -1 -24 3 glass", "fill -3 -26 1 3 -24 1 glass", "tp @s 0.5 -26 0.5"].map(String::from));
    }
    for at in ["6.5 -26 0.5", "0.5 -26 -5.5"] {
        lines.extend([format!("summon husk {at} 0 0"), "effect @s instant_health 1 4".into()]);
        waits(&mut lines, 5);
        lines.extend(["kill @e[type=husk]".into(), "tp @s 0.5 -26 0.5".into()]);
    }
    with(lines, &["fill -3 -26 -3 3 -24 3 air"])
}

/// A spider 6 off by night for 10 s, then noon for 10 s; one summoned at noon 6 off for 8 s.
fn spider() -> Vec<String> {
    let mut lines = with(quiet("midnight"), &["summon spider 6.5 -26 0.5 0 0"]);
    waits(&mut lines, 10);
    lines.push("time set noon".into());
    waits(&mut lines, 10);
    lines.extend(["kill @e[type=spider]".into(), "tp @s 0.5 -26 0.5".into(), "summon spider 6.5 -26 0.5 0 0".into()]);
    waits(&mut lines, 8);
    with(lines, &["kill @e[type=spider]"])
}

/// Four gold nuggets dropped toward a piglin armed by its spawn, then a bare (`spawn_adult`) one.
fn nugget() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "tp @s 0.5 -26 0.5 -90 0"]);
    for event in ["", " minecraft:spawn_adult"] {
        lines.extend([format!("summon piglin 3.5 -26 0.5 90 0{event}"), "replaceitem entity @s slot.hotbar 0 gold_nugget 4".into(), String::new(), "!drop 0 4".into()]);
        waits(&mut lines, 4);
        lines.extend(["kill @e[type=piglin]".into(), "kill @e[type=item]".into()]);
    }
    lines
}

/// The actor holding wheat; a cow summoned 5, 9, 11 and 14 off in x, 6 s each.
fn tempt() -> Vec<String> {
    let mut lines = with(quiet("noon"), &["replaceitem entity @s slot.weapon.mainhand 0 wheat 8"]);
    for off in [5, 9, 11, 14] {
        lines.push(format!("summon cow {}.5 -26 0.5 90 0", off));
        waits(&mut lines, 6);
        lines.push("kill @e[type=cow]".into());
    }
    with(lines, &["gamerule domobloot true", "clear @s"])
}
