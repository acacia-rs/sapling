//! The fourteenth wave's group U drills (research/parity/FOURTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27). `u3chest`: plain chests paired by hand and by `/setblock`, opened at either half, shut, and
//! one taken away while open; then a copper pair opened at its second half. `u3carry`: a stand, a cow and sand on
//! stones pushed east, sand and a stand on stones a sticky piston draws back; slime, honey and stone pushed east
//! with air under them and a stand or a cow on each. `u3plants`: twisting vines, standing and hanging dripstone,
//! weeping vines and kelp set on or under fourteenth1i's supporters, then nudged by a neighbour.

use super::creatures::{floor, waits, with};
use super::fourteenth1i::{FIRST, LAST, reset, supporters};

const BLOCKS: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "BlockEvent", "ContainerOpen", "ContainerClose", "LevelEvent", "LevelSoundEvent", "AddItemEntity", "CommandOutput"];
const MOVES: &[&str] = &[
    "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "UpdateBlock", "UpdateSubchunkBlocks", "LevelEvent", "LevelSoundEvent", "AddItemEntity", "CommandOutput",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "u3chest" => (chests(), BLOCKS),
        "u3carry" => (carry(), MOVES),
        "u3plants" => (plants(), BLOCKS),
        "u3sched" => (scheduled(), &["CommandOutput", "Text"]),
        "u3yaw" => (curve(), MOVES),
        "u3place" => (rules_and_options(), &["CommandOutput", "UpdateBlock", "UpdateSubchunkBlocks", "AddEntity"]),
        _ => return None,
    })
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

fn tidy(lines: Vec<String>) -> Vec<String> {
    with(lines, &["", "gamemode creative @s", "fill -8 -26 -8 16 -16 8 air", "kill @e[type=!player]", "tp @s 0.5 -26 0.5 -90 0", ""])
}

/// Opens the chest half at the offset from the actor's feet, waits, shuts it.
fn open_shut(lines: &mut Vec<String>, at: &str) {
    lines.extend([hold("air"), format!("!click {at} up"), String::new(), String::new(), "!close".into(), String::new(), String::new()]);
}

fn chests() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", &hold("chest 1"), "!place 2 -1 0 up", "", &hold("chest 1"), "!place 2 -1 1 up", "", ""]);
    open_shut(&mut lines, "2 0 0");
    open_shut(&mut lines, "2 0 1");
    lines = with(lines, &["setblock 2 -26 -3 chest [\"minecraft:cardinal_direction\"=\"west\"]", "setblock 2 -26 -2 chest [\"minecraft:cardinal_direction\"=\"west\"]", "", ""]);
    open_shut(&mut lines, "2 0 -2");
    lines = with(lines, &[&hold("air"), "!click 2 0 -3 up", "", "", "setblock 2 -26 -3 air", "", "", "!close", ""]);
    lines = with(lines, &[&hold("copper_chest 1"), "!place 2 -1 3 up", "", &hold("copper_chest 1"), "!place 2 -1 4 up", "", ""]);
    open_shut(&mut lines, "2 0 4");
    tidy(lines)
}

fn piston(kind: &str, x: i32, y: i32, z: i32) -> String {
    format!("setblock {x} {y} {z} {kind} [\"facing_direction\"=4]")
}

const STILL: &str = "effect @e[type=cow] slowness 100 255 true";

fn carry() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "tp @s -6.5 -26 -6.5 -90 0"]);
    for z in [-6, -4, -2] {
        lines.extend([piston("piston", 2, -26, z), format!("setblock 3 -26 {z} stone")]);
    }
    lines = with(lines, &["summon armor_stand 3.5 -25 -5.5", "summon cow 3.5 -25 -3.5", "setblock 3 -25 -2 sand", STILL, "", "fill 2 -25 -6 2 -25 -2 redstone_block"]);
    waits(&mut lines, 3);
    lines = with(lines, &["fill 2 -25 -6 2 -25 -2 air"]);
    waits(&mut lines, 3);
    for z in [0, 2] {
        lines.extend([piston("sticky_piston", 2, -26, z), format!("setblock 3 -26 {z} stone")]);
    }
    lines = with(lines, &["", "fill 2 -25 0 2 -25 2 redstone_block"]);
    waits(&mut lines, 2);
    lines = with(lines, &["setblock 4 -25 0 sand", "summon armor_stand 4.5 -25 2.5", "", "fill 2 -25 0 2 -25 2 air"]);
    waits(&mut lines, 3);
    // Raised on stone columns, the pushed blocks with air under them.
    for (z, block) in [(-6, "slime"), (-4, "slime"), (-2, "honey_block"), (0, "stone")] {
        lines.extend([format!("fill 8 -26 {z} 8 -25 {z} stone"), piston("piston", 8, -24, z), format!("setblock 9 -24 {z} {block}")]);
    }
    lines = with(lines, &["summon armor_stand 9.5 -23 -5.5", "summon cow 9.5 -23 -3.5", "summon armor_stand 9.5 -23 -1.5", "summon armor_stand 9.5 -23 0.5", STILL, "", "fill 8 -23 -6 8 -23 0 redstone_block"]);
    waits(&mut lines, 5);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 stone", "tp @s 0.5 -26 0.5 -90 0", ""])
}

/// Sets `block` over the row at `y`, z 2, then nudges it by a stone set and taken away at z 3.
fn nudged(lines: &mut Vec<String>, y: i32, block: &str) {
    lines.extend([format!("fill {FIRST} {y} 2 {LAST} {y} 2 {block}"), String::new(), format!("fill {FIRST} {y} 3 {LAST} {y} 3 stone"), format!("fill {FIRST} {y} 3 {LAST} {y} 3 air"), String::new()]);
}

fn plants() -> Vec<String> {
    let mut lines = supporters();
    lines.extend(reset());
    nudged(&mut lines, -25, "twisting_vines");
    lines.extend(reset());
    nudged(&mut lines, -25, "pointed_dripstone [\"hanging\"=false]");
    // Hung under the supporters lifted two.
    lines.extend(reset());
    lines.extend([format!("clone {FIRST} -26 2 {LAST} -26 2 {FIRST} -24 2"), format!("fill {FIRST} -26 2 {LAST} -26 2 air")]);
    nudged(&mut lines, -25, "weeping_vines");
    lines.push(format!("fill {FIRST} -25 2 {LAST} -25 2 air"));
    nudged(&mut lines, -25, "pointed_dripstone [\"hanging\"=true]");
    lines.push(format!("fill {FIRST} -25 2 {LAST} -24 2 air"));
    // Kelp in a glass trough of water over them.
    lines.extend(reset());
    lines.extend([
        format!("fill {} -25 1 {} -25 3 glass", FIRST - 1, LAST + 1),
        format!("fill {FIRST} -25 2 {LAST} -25 2 water"),
        String::new(),
        format!("fill {FIRST} -25 2 {LAST} -25 2 kelp"),
        String::new(),
        format!("fill {FIRST} -25 3 {LAST} -25 3 stone"),
        String::new(),
        String::new(),
        format!("fill {} -25 1 {} -25 3 air", FIRST - 1, LAST + 1),
    ]);
    lines.extend(reset());
    lines
}

/// A minecart sent by powered rails along z 8 into a curve north (`rail_direction` 8), for its yaw.
fn curve() -> Vec<String> {
    let mut lines = with(floor(), &[
        "setblock -8 -26 8 stone",
        "fill -7 -27 8 -5 -27 8 redstone_block",
        "fill -7 -26 8 -5 -26 8 golden_rail [\"rail_direction\"=1]",
        "fill -4 -26 8 3 -26 8 rail [\"rail_direction\"=1]",
        "setblock 4 -26 8 rail [\"rail_direction\"=8]",
        "fill 4 -26 7 4 -26 -6 rail [\"rail_direction\"=0]",
        "",
        "summon minecart -6.5 -26 8.5",
    ]);
    waits(&mut lines, 6);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 stone", ""])
}

/// `/schedule on_area_loaded` on a loaded area, then on far ones a ticking area later loads; needs
/// tools/bds-packs-setup.sh's `probe/later` function and KEEP_WORLD=1.
fn scheduled() -> Vec<String> {
    let mut lines = with(Vec::new(), &["schedule on_area_loaded add 0 -30 0 1 -20 1 probe/later", "", "schedule on_area_loaded add 3000 -30 3000 3001 -20 3001 probe/later", "", "", "", "tickingarea add 3000 -30 3000 3001 -20 3001 u3area"]);
    waits(&mut lines, 5);
    lines = with(lines, &["tickingarea remove u3area", "schedule on_area_loaded add circle 5000 0 5000 2 probe/later", "schedule on_area_loaded add tickingarea u3none probe/later", "tickingarea add circle 5000 0 5000 2 u3circle"]);
    waits(&mut lines, 5);
    with(lines, &["tickingarea remove u3circle", "schedule on_area_loaded add 7000 0 7000 7001 0 7001 probe/later", "schedule clear probe/later", "schedule on_area_loaded add 7000 0 7000 7001 0 7001 probe/nonesuch", "tickingarea add 7000 0 7000 7001 0 7001 u3late", "", "", "", "tickingarea remove u3late", ""])
}

const RULES: &[&str] = &[
    "overworld_surface_tall_grass_feature",
    "overworld_after_surface_flowers_feature_rules",
    "plains_surface_flowers_feature",
    "overworld_underground_coal_ore_feature",
    "overworld_underground_dirt_feature",
    "overworld_amethyst_geode_feature",
];

/// `/place featurerule` and `/place structure`'s options where the actor lands at (30, 210) on a generated
/// world of seed 20261003 (BDS copy of level-type DEFAULT, op level 4; sapling `SAPLING_DRILL_WORLD=generated`).
fn rules_and_options() -> Vec<String> {
    // Sapling's drill server is still generating its spawn the first seconds and drops commands.
    let mut lines = Vec::new();
    waits(&mut lines, 15);
    lines = with(lines, &["gamerule domobspawning false", "gamemode creative @s", "tp @s 30 120 210"]);
    waits(&mut lines, 10);
    for rule in RULES {
        lines.extend([format!("place featurerule minecraft:{rule} ~ ~ ~"), format!("place featurerule minecraft:{rule} ~ ~-1 ~"), format!("place featurerule minecraft:{rule} ~ ~-30 ~"), String::new()]);
    }
    for options in ["~40 ~ ~ true", "~-40 ~ ~ false", "~ ~ ~40 false false true", "~ ~ ~-40 false false false"] {
        lines.extend([format!("place structure minecraft:trail_ruins {options}"), String::new(), String::new()]);
    }
    lines.extend(["place structure minecraft:abandoned_camp_forest ~60 ~ ~60 false false true".into(), String::new(), String::new()]);
    with(lines, &["place structure minecraft:abandoned_camp_forest ~-60 ~ ~-60 false false false", "", ""])
}
