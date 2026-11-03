//! Drills of the third wave's mounts (research/parity/MORE-BRIEF.md "Group 2"): the saddled spawn
//! events, a saddled pig's and strider's death, and which mounts `/replaceitem` saddles or armours.
//! On creature_drill.rs's floor.

use super::creatures::floor;

/// Packets shown of the saddle drills.
const SHOWN: &[&str] = &["AddEntity", "SetEntityData", "LevelSoundEvent", "MobArmorEquipment", "AddItemEntity"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "saddleevents" => (saddle_events(), SHOWN),
        "saddlerule" => (saddle_rule(), SHOWN),
        _ => return None,
    })
}

/// A held-still `kind` summoned at `z` by `event` (none: plain).
fn summoned(lines: &mut Vec<String>, kind: &str, z: i32, event: Option<&str>) {
    let event = event.map_or(String::new(), |event| format!(" 0 0 minecraft:{event}"));
    lines.extend([format!("summon {kind} 4.5 -26 {z}.5{event}"), format!("effect @e[type={kind}] slowness 200 255 true"), String::new(), String::new()]);
}

/// Killed alone, its drops shown, then cleared.
fn killed(lines: &mut Vec<String>, kind: &str) {
    lines.extend([format!("kill @e[type={kind}]"), String::new(), String::new(), "kill @e[type=item]".into(), String::new()]);
}

/// Each mount summoned by its saddled event, then killed; a pig and a strider saddled by event,
/// then by the actor's click with a saddle, killed with `doMobLoot` on, last a pig with it off;
/// `slot.saddle` on a pig.
fn saddle_events() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["gamerule domobloot true", ""].map(String::from));
    for (kind, event) in [("horse", "horse_saddled"), ("donkey", "donkey_saddled"), ("mule", "mule_saddled"), ("zombie_horse", "horse_saddled"), ("camel", "camel_saddled"), ("pig", "on_saddled"), ("strider", "on_saddled")] {
        summoned(&mut lines, kind, 0, Some(event));
        killed(&mut lines, kind);
    }
    for kind in ["pig", "strider"] {
        summoned(&mut lines, kind, 0, None);
        lines.extend(["clear @s".to_owned(), "give @s saddle".into(), "tp @s 2.5 -26 0.5 -90 0".into(), String::new(), format!("!interact {kind}"), String::new()]);
        killed(&mut lines, kind);
    }
    summoned(&mut lines, "pig", 0, None);
    lines.extend(["replaceitem entity @e[type=pig] slot.saddle 0 saddle", "", "kill @e[type=pig]", "", ""].map(String::from));
    lines.extend(["gamerule domobloot false".to_owned(), String::new()]);
    summoned(&mut lines, "pig", 0, Some("on_saddled"));
    killed(&mut lines, "pig");
    lines.extend(["gamerule domobloot true", "clear @s"].map(String::from));
    lines
}

/// `slot.saddle` then `slot.armor.body` on wild, tamed and baby mounts, each alone: which take them,
/// and what shows.
fn saddle_rule() -> Vec<String> {
    let mut lines = floor();
    let cases = [
        ("horse", Some("spawn_adult")),
        ("horse", Some("entity_born")),
        ("donkey", Some("spawn_adult")),
        ("mule", Some("spawn_adult")),
        ("zombie_horse", Some("spawn_adult")),
        ("zombie_horse", Some("spawn_tame_adult")),
        ("zombie_horse", Some("entity_born")),
        ("skeleton_horse", None),
        ("skeleton_horse", Some("entity_born")),
        ("camel", Some("entity_born")),
        ("llama", Some("spawn_adult")),
    ];
    for (kind, event) in cases {
        summoned(&mut lines, kind, 0, event);
        let armor = if kind == "llama" { "white_carpet" } else { "iron_horse_armor" };
        lines.extend([format!("replaceitem entity @e[type={kind}] slot.saddle 0 saddle"), String::new(), format!("replaceitem entity @e[type={kind}] slot.armor.body 0 {armor}"), String::new()]);
        killed(&mut lines, kind);
    }
    lines
}
