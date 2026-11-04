//! The fifth wave's mob drills (research/parity/FIFTH-BRIEF.md "Group 1"): mobs left wearing body
//! armour for the world folder to be read (`bodysave`, sapling-leveldb's `actor_dump`), and slowed
//! mobs near and far from the bots turning their heads (`gaze`).

use super::creatures::{floor, waits, with};

const BODY_SHOWN: &[&str] = &["AddEntity", "SetEntityData", "MobArmorEquipment", "MobEquipment"];
const GAZE_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "bodysave" => (body_save(), BODY_SHOWN),
        "gaze" => (gaze(), GAZE_SHOWN),
        _ => return None,
    })
}

/// Each mob: its kind and spawn event, where it stands, and what `/replaceitem` puts on it in order.
const KEPT: &[(&str, &str, [&str; 3], &[&str])] = &[
    ("horse", " 0 0 minecraft:spawn_tame_adult", ["4.5", "-26", "0.5"], &["slot.armor.body 0 iron_horse_armor"]),
    ("donkey", " 0 0 minecraft:spawn_tame_adult", ["4.5", "-26", "3.5"], &["slot.armor.body 0 diamond_horse_armor"]),
    ("horse", " 0 0 minecraft:spawn_tame_adult", ["4.5", "-26", "-3.5"], &["slot.armor 0 golden_horse_armor", "slot.armor.body 0 diamond_horse_armor"]),
    ("horse", " 0 0 minecraft:spawn_tame_adult", ["0.5", "-26", "-3.5"], &["slot.armor 0 golden_horse_armor"]),
    ("horse", " 0 0 minecraft:spawn_tame_adult", ["0.5", "-26", "3.5"], &["slot.armor.body 0 diamond_horse_armor", "slot.armor 0 golden_horse_armor"]),
    ("pig", "", ["8.5", "-26", "0.5"], &["slot.armor.body 0 iron_horse_armor"]),
    ("wolf", "", ["8.5", "-26", "3.5"], &["slot.armor.body 0 wolf_armor"]),
    ("husk", "", ["8.5", "-26", "-3.5"], &["slot.armor.head 0 diamond_helmet", "slot.armor.body 0 iron_horse_armor"]),
];

/// The mobs of [`KEPT`] dressed and held, left standing when the bots go.
fn body_save() -> Vec<String> {
    let mut lines = floor();
    for (kind, event, [x, y, z], puts) in KEPT {
        let one = format!("@e[type={kind},x={x},y={y},z={z},r=1]");
        lines.extend([format!("summon {kind} {x} {y} {z}{event}"), format!("effect {one} slowness 1000 255 true"), String::new()]);
        lines.extend(puts.iter().map(|put| format!("replaceitem entity {one} {put}")));
    }
    waits(&mut lines, 3);
    lines
}

/// Three slowed cows 3 blocks from the bots (within `look_at_player`'s 6) and three 20 off, on the
/// flat ground east of the floor, watched 40 s.
fn gaze() -> Vec<String> {
    let mut lines = with(floor(), &["tp @s 52.5 -60 0.5", "tp CmdWatcher 52.5 -60 2.5", ""]);
    for at in ["55.5 -60 -1.5", "55.5 -60 1.5", "55.5 -60 4.5", "72.5 -60 -1.5", "72.5 -60 1.5", "72.5 -60 4.5"] {
        lines.push(format!("summon cow {at}"));
    }
    lines.push("effect @e[type=cow] slowness 1000 255 true".into());
    waits(&mut lines, 40);
    with(lines, &["kill @e[type=cow]", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}
