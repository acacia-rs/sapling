//! The seventh wave's packet drills (research/parity/SEVENTH-BRIEF.md "Group 3"): the entity each
//! block gets at `/setblock` and what is sent again a tick later (`s7ents`), summoned projectiles'
//! flight and flag words (`s7shots`), and mounts summoned with events their kind may lack
//! (`s7mounts`).

use super::creatures::{floor, waits, with};

const ENTS_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData"];
const SHOTS_SHOWN: &[&str] = &["AddEntity", "SetEntityData", "MoveEntityDelta", "SetEntityMotion", "RemoveEntity", "LevelEvent", "LevelSoundEvent"];
const MOUNTS_SHOWN: &[&str] = &["AddEntity", "SetEntityData", "LevelSoundEvent", "RemoveEntity"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "s7ents" => (set_entities(), ENTS_SHOWN),
        "s7shots" => (summoned_shots(), SHOTS_SHOWN),
        "s7mounts" => (mounts(), MOUNTS_SHOWN),
        _ => return None,
    })
}

/// Blocks that may get an entity at `/setblock`, with the states that keep them standing.
const SET: &[&str] = &[
    "frame [\"facing_direction\"=1]",
    "glow_frame [\"facing_direction\"=1]",
    "lectern",
    "jukebox",
    "lodestone",
    "ender_chest",
    "mob_spawner",
    "trial_spawner",
    "vault",
    "standing_sign",
    "oak_hanging_sign [\"attached_bit\"=true,\"hanging\"=false]",
    "campfire",
    "conduit",
    "standing_banner",
    "furnace",
    "hopper",
    "brewing_stand",
    "enchanting_table",
    "noteblock",
    "daylight_detector",
    "unpowered_comparator",
    "barrel",
    "undyed_shulker_box",
    "dispenser",
    "crafter",
    "beehive",
    "sculk_sensor",
    "sculk_catalyst",
    "sculk_shrieker",
    "decorated_pot",
    "piston",
    "command_block",
    "creaking_heart",
    "suspicious_sand",
    "cauldron",
    "skeleton_skull [\"facing_direction\"=1]",
    "bed",
    "flower_pot",
];

/// Each block in a cell of its own, west to east in rows two apart, away from the bots.
fn set_entities() -> Vec<String> {
    let mut lines = floor();
    for (at, block) in SET.iter().enumerate() {
        let (x, z) = (-6 + 2 * (at % 11) as i32, [-7, -4, 4, 7][at / 11]);
        lines.push(format!("setblock {x} -26 {z} {block}"));
    }
    waits(&mut lines, 1);
    lines
}

/// Projectiles summoned still, in the air over the floor, one at a time; then TNT for its flags.
const SHOTS: &[&str] = &["arrow", "snowball", "egg", "ender_pearl", "thrown_trident", "splash_potion", "xp_bottle"];

fn summoned_shots() -> Vec<String> {
    let mut lines = floor();
    for (at, kind) in SHOTS.iter().enumerate() {
        let x = -6.5 + 3.0 * at as f32;
        lines.extend([format!("summon {kind} {x} -22 -5.5"), String::new(), String::new()]);
    }
    lines.extend(["kill @e[type=!player]".to_owned(), String::new()]);
    waits(&mut lines, 1);
    lines
}

/// Mounts summoned with events their pack lacks or that add no age group, and the kill after.
const MOUNTS: &[(&str, &str)] = &[
    ("trader_llama", "spawn_tame_adult"),
    ("skeleton_horse", "spawn_tame_adult"),
    ("skeleton_horse", "spawn_adult"),
    ("zombie_horse", "horse_saddled"),
    ("horse", "on_tame"),
    ("camel", "camel_saddled"),
    ("camel", "spawn_adult"),
    ("mule", "on_tame"),
];

fn mounts() -> Vec<String> {
    let mut lines = with(floor(), &["time set midnight"]);
    for (kind, event) in MOUNTS {
        lines.extend([format!("summon {kind} 4.5 -26 -4.5 0 0 minecraft:{event}"), format!("effect @e[type={kind}] slowness 200 255 true"), String::new(), String::new()]);
        lines.extend([format!("kill @e[type={kind}]"), String::new(), "kill @e[type=item]".into(), String::new()]);
    }
    with(lines, &["time set noon"])
}
