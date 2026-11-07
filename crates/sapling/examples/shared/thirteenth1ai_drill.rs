//! The thirteenth wave's group AI drills (research/parity/THIRTEENTH-BRIEF.md): when a vibration's
//! flight is sent (`ai1vib`), `/fill` over blocks that react to their neighbours (`ai1fill`), boats
//! summoned one at a time and held under water (`ai1boat`), and a ridden camel and horse (`ai1ride`).

use crate::creatures::{floor, waits, with};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ai1vib" => (vibrations(), VIB_SHOWN),
        "ai1fill" => (fills(), FILL_SHOWN),
        "ai1boat" => (boats(), BOAT_SHOWN),
        "ai1ride" => (rides(), RIDE_SHOWN),
        "ai1camps" => (camps(), CAMP_SHOWN),
        "ai1liquid" => (liquids(), FILL_SHOWN),
        _ => return None,
    })
}

const VIB_SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEventGeneric", "CommandOutput", "AddItemEntity"];

/// A sculk sensor 5 or more off hears, each after it has settled: a stone put by hand, a torch
/// broken by hand (survival), a `/tp`, a walk, and a water bucket poured.
fn vibrations() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "setblock 6 -26 4 sculk_sensor", "replaceitem entity @s slot.weapon.mainhand 0 stone 8", ""]);
    waits(&mut lines, 2);
    let settle = |lines: &mut Vec<String>| waits(lines, 3);
    lines.extend(["!place 1 -1 1", ""].map(String::from));
    settle(&mut lines);
    lines.extend(["setblock 1 -26 3 torch", "", "", "gamemode survival @s", "!break 1 0 3", "", "gamemode creative @s"].map(String::from));
    settle(&mut lines);
    lines.extend(["tp @s 1.2 -26 3.8 0 0", ""].map(String::from));
    settle(&mut lines);
    lines.extend(["tp @s -4.5 -26 4.5 -90 0", ""].map(String::from));
    settle(&mut lines);
    lines.extend(["!walk", "", "!stop"].map(String::from));
    settle(&mut lines);
    lines.extend(["tp @s 0.5 -26 0.5 -90 0", "replaceitem entity @s slot.weapon.mainhand 0 water_bucket", "", "!click 2 -1 0", ""].map(String::from));
    settle(&mut lines);
    with(lines, &["fill -8 -26 -8 16 -16 8 air", "clear @s", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

const FILL_SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "AddEntity", "RemoveEntity", "AddItemEntity", "LevelSoundEvent", "LevelEvent", "CommandOutput"];

/// `/fill` over blocks that react to neighbours, each beside the same done by `/setblock`s: a
/// floating line of sand, rails in a line and into a corner, redstone dust from a redstone block,
/// water on the floor, and a torch's support filled away.
fn fills() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "tp @s -6.5 -26 -5.5", ""]);
    lines.extend(["fill 2 -22 2 4 -22 2 sand", ""].map(String::from));
    waits(&mut lines, 2);
    lines.extend(["setblock 2 -22 6 sand", "setblock 3 -22 6 sand", "setblock 4 -22 6 sand", ""].map(String::from));
    waits(&mut lines, 2);
    lines.extend(["fill 2 -26 -2 6 -26 -2 rail", "", "fill 8 -26 -6 8 -26 -2 rail", "fill 9 -26 -2 12 -26 -2 rail", ""].map(String::from));
    lines.extend(["setblock 2 -26 -4 rail", "setblock 3 -26 -4 rail", "setblock 4 -26 -4 rail", ""].map(String::from));
    lines.extend(["setblock 2 -26 -7 redstone_block", "fill 3 -26 -7 7 -26 -7 redstone_wire", ""].map(String::from));
    lines.extend(["setblock 9 -26 -7 redstone_block", "setblock 10 -26 -7 redstone_wire", "setblock 11 -26 -7 redstone_wire", "setblock 12 -26 -7 redstone_wire", ""].map(String::from));
    waits(&mut lines, 1);
    lines.extend(["fill 7 -26 4 9 -26 4 water", ""].map(String::from));
    waits(&mut lines, 3);
    lines.extend(["setblock 14 -26 4 torch", "", "fill 13 -27 4 15 -27 4 glass", ""].map(String::from));
    lines.extend(["setblock 14 -26 6 torch", "", "setblock 14 -27 6 glass", ""].map(String::from));
    waits(&mut lines, 2);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 stone", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

/// Water and lava set by one `/setblock` and filled one cell and three in a row on the open floor,
/// each left 4 s: which of them flow (sapling's last filled water cell stood still in `ai1fill`).
fn liquids() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "tp @s -6.5 -26 -5.5", ""]);
    for (n, line) in ["setblock 2 -26 -4 water", "fill 2 -26 2 2 -26 2 water", "fill 8 -26 -4 10 -26 -4 water", "fill 8 -26 2 8 -26 4 water", "setblock 13 -26 -4 lava", "fill 13 -26 2 15 -26 2 lava"].iter().enumerate() {
        lines.extend([line.to_string(), String::new()]);
        waits(&mut lines, 3);
        if n % 2 == 1 {
            lines.extend(["fill -8 -26 -8 16 -24 8 air".to_owned(), String::new()]);
        }
    }
    with(lines, &["fill -8 -26 -8 16 -16 8 air", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

const BOAT_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "SetEntityLink", "SetEntityMotion", "MoveEntityDelta", "MoveEntityAbsolute", "EntityEvent", "SetEntityData", "CommandOutput"];

/// Boats summoned one at a time on the floor, the players well off, to see the summoned shift
/// alone; then a boat with a pig in it held under a stone roof in a pool for 12 s.
fn boats() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "tp @s -6.5 -26 -5.5", ""]);
    for (x, z) in [(4.5, 4.5), (2.0, 2.0), (6.3, 2.8), (9.5, 4.5), (3.5, -2.5), (8.0, -3.0), (12.25, 0.75), (10.9, 5.9)] {
        lines.push(format!("summon boat {x} -26 {z}"));
        waits(&mut lines, 2);
        lines.push("kill @e[type=boat]".into());
    }
    lines.extend(["kill @e[type=item]", "fill 1 -31 -7 7 -27 -1 stone", "fill 2 -30 -6 6 -27 -2 water", "fill 1 -26 -7 7 -26 -1 stone", "summon boat 4.5 -29 -4.5", "summon pig 4.5 -28 -4.5", "", "ride @e[type=pig] start_riding @e[type=boat,c=1]", ""].map(String::from));
    waits(&mut lines, 12);
    with(lines, &["kill @e[type=!player]", "fill -8 -31 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 stone", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

const RIDE_SHOWN: &[&str] = &["AddEntity", "SetEntityLink", "SetEntityMotion", "MoveEntityDelta", "MoveEntityAbsolute", "MovePlayer", "SetEntityData", "EntityEvent", "LevelSoundEvent", "CorrectPlayerMovePrediction", "UpdateAttributes", "CommandOutput"];

/// The actor rides a saddled camel south 3 s, then holds jump 10 ticks (its dash); then a tamed,
/// saddled horse south 2 s.
fn rides() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 8.5 -26 -7.5", "tp @s 0.5 -26 -7.5 0 0", ""]);
    lines.extend(["summon camel 0.5 -26 -6.5 0 0 minecraft:camel_saddled", "", "", "!mount camel", "", "!look 0 1.6 8", "!walk", "", "", "", "!stop", "", "!charge 10", "", "", "!dismount", ""].map(String::from));
    lines.extend(["kill @e[type=!player]", "tp @s 4.5 -26 -7.5 0 0", "summon horse 4.5 -26 -6.5 0 0 minecraft:on_tame", "", "replaceitem entity @e[type=horse] slot.saddle 0 saddle", "", "!mount horse", "", "!look 0 1.6 8", "!walk", "", "", "!stop", "", "!dismount", ""].map(String::from));
    with(lines, &["kill @e[type=!player]", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

const CAMP_SHOWN: &[&str] = &["CommandOutput"];

const CAMPS: [&str; 18] = [
    "bamboo_jungle", "birch_forest", "cherry_grove", "dappled_forest", "flower_forest", "forest", "meadow", "birch_forest_mutated", "mega_taiga",
    "redwood_taiga_mutated", "pale_garden", "savanna", "cold_taiga", "jungle_edge", "swampland", "taiga", "extreme_hills_plus_trees", "mesa_plateau_stone",
];

/// `/place structure` of every abandoned camp variant, 64 apart over the flat ground, then the
/// forest one 9000 off (unloaded); `/place jigsaw` with a target its pool has and two it lacks.
fn camps() -> Vec<String> {
    let mut lines = vec!["gamerule domobspawning false".to_owned(), String::new()];
    for (n, camp) in CAMPS.iter().enumerate() {
        lines.extend([format!("place structure minecraft:abandoned_camp_{camp} {} -60 200", n as i32 * 64 - 576), String::new()]);
    }
    lines.extend(["place structure minecraft:abandoned_camp_forest 9000 -60 9000", "", "place structure minecraft:abandoned_camp_meadow 9000 -60 9200", ""].map(String::from));
    for target in ["road_anchor", "bottom", "no_such_target"] {
        lines.extend([format!("place jigsaw minecraft:trail_ruins/tower minecraft:{target} 3 -300 -60 -300"), String::new()]);
    }
    lines
}
