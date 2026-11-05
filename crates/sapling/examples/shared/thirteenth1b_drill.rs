//! The thirteenth wave's group F drills (research/parity/THIRTEENTH-BRIEF.md): when a survival
//! player beside a stuck dispenser arrow picks it up (`f1pickup`), and a player standing under a
//! block one high (`f1crawl`), and command answers REMAINING3 lists (`f1cmds`).

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "MoveEntityDelta", "SetEntityMotion", "EntityEvent", "TakeItemEntity", "InventorySlot", "InventoryContent", "LevelSoundEvent", "CommandOutput",
];
const CMDS_SHOWN: &[&str] = &["AddEntity", "SetEntityData", "SetEntityLink", "CommandOutput"];
const POSE_SHOWN: &[&str] =&["SetEntityData", "UpdateAttributes", "MovePlayer", "CorrectPlayerMovePrediction", "EntityEvent", "CommandOutput"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "f1pickup" => (pickup(), SHOWN),
        "f1crawl" => (crawl(), POSE_SHOWN),
        "f1cmds" => (commands(), CMDS_SHOWN),
        _ => return None,
    })
}

/// A dispenser at x -3 shoots an arrow east along `z` into stone at x 3; the survival actor
/// stands 1 south of where it sticks, within reach.
fn shot_beside(lines: &mut Vec<String>, z: i32) {
    lines.extend([
        format!("tp @s 2.5 -26 {}.5", z + 1),
        format!("setblock 3 -26 {z} stone"),
        format!("setblock 3 -25 {z} stone"),
        format!("setblock -3 -26 {z} dispenser [\"facing_direction\"=5]"),
        format!("replaceitem block -3 -26 {z} slot.container 0 arrow 1"),
        String::new(),
        format!("setblock -3 -25 {z} redstone_block"),
    ]);
    waits(lines, 4);
}

fn pickup() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "effect @s resistance 60 4 true", ""]);
    shot_beside(&mut lines, 0);
    shot_beside(&mut lines, -4);
    with(lines, &["gamemode creative @s", "effect @s clear", "kill @e[type=!player]", "fill -4 -26 -5 4 -24 2 air", ""])
}

/// The survival actor taken under a stone ceiling one block over the floor, then out again.
fn crawl() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "effect @s resistance 60 4 true", "fill 2 -25 -1 6 -25 1 stone", "", "tp @s 4.5 -26 0.5"]);
    waits(&mut lines, 4);
    lines.push("tp @s 0.5 -26 0.5".into());
    waits(&mut lines, 2);
    with(lines, &["gamemode creative @s", "effect @s clear", "fill 2 -25 -1 6 -25 1 air", ""])
}

/// REMAINING3's command lines: numbers past i32, `^=`, a summon's pitch, a summoned rider's name
/// tag and `tickingarea list all-dimensions`.
fn commands() -> Vec<String> {
    let lines = with(floor(), &[
        "scoreboard objectives add f1 dummy",
        "scoreboard players set @s f1 3000000000",
        "scoreboard players add @s f1 -2147483649",
        "scoreboard players set @s f1 5",
        "scoreboard players operation @s f1 ^= @s f1",
        "summon pig 2 -26 2 90 45",
        "",
        "ride @e[type=pig] summon_rider zombie minecraft:entity_spawned Rider",
        "",
        "ride @s summon_ride horse no_ride_change minecraft:entity_spawned Ride",
        "",
        "tickingarea add 0 0 0 0 0 0 f1area",
        "tickingarea list all-dimensions",
        "tickingarea remove f1area",
        "scoreboard objectives remove f1",
    ]);
    with(lines, &["ride @s stop_riding", "kill @e[type=!player]", ""])
}
