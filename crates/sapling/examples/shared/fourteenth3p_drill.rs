//! The fourteenth wave's group P block drills (research/parity/FOURTEENTH-BRIEF.md), on creature_drill.rs's
//! floor (stone at y -27). `p3ground`: dust, rails, repeaters and comparators put by hand and by `/fill` on
//! fourteenth1i's supporters. `p3sculk`: sculk sensors and a shrieker put by hand. `p3carpet`: carpet set over
//! air by `/setblock` and `/fill`. `p3plate`: plates whose support goes. `p3snow`: sand dropped on snow
//! layers over leaves and stone. `p3anvil`: damaged anvils dropped. `p3bell`: bells put in one-wide gaps.
//! `p3names`: vehicles summoned with a name. `p3lava`: carts summoned on lava and water. `p3pot`: decorated
//! pots shot by a dispenser. `p3disp`: dispensed arrows on normal and hard. `p3copper`: copper chests, one
//! alone and a pair, waxed, scraped and random-ticked. `p3alias`: `chain` and `anvil`'s `damage` set by their old names.

use super::creatures::{floor, waits, with};
use super::fourteenth1i::{FIRST, LAST, reset, supporters};

const BLOCKS: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "UpdateBlockSynced", "BlockEntityData", "LevelEvent", "LevelSoundEvent", "AddEntity", "AddItemEntity", "RemoveEntity", "CommandOutput"];
const MOVES: &[&str] = &["AddEntity", "AddItemEntity", "SetEntityMotion", "MoveEntityDelta", "MoveEntityAbsolute", "RemoveEntity", "SetEntityData", "EntityEvent", "LevelEvent", "LevelSoundEvent", "UpdateBlock", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "p3ground" => (ground(), BLOCKS),
        "p3sculk" => (sculk(), BLOCKS),
        "p3carpet" => (carpet(), BLOCKS),
        "p3plate" => (plate(), BLOCKS),
        "p3snow" => (snow(), BLOCKS),
        "p3anvil" => (anvil(), BLOCKS),
        "p3bell" => (bell(), BLOCKS),
        "p3names" => (names(), MOVES),
        "p3lava" => (lava(), MOVES),
        "p3pot" => (pot(), MOVES),
        "p3disp" => (dispensed(), MOVES),
        "p3copper" => (copper(), BLOCKS),
        "p3alias" => (alias(), BLOCKS),
        _ => return None,
    })
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

fn owned<const N: usize>(lines: [&str; N]) -> impl Iterator<Item = String> {
    lines.into_iter().map(String::from)
}

fn tidy(lines: Vec<String>) -> Vec<String> {
    with(lines, &["", "fill -8 -26 -8 16 -16 8 air", "kill @e[type=!player]", "tp @s 0.5 -26 0.5 -90 0", ""])
}

/// Dust, a rail, a repeater and a comparator put by hand on each supporter's top, then each set by
/// `/fill` over them all and nudged by a stone put and taken away beside them.
fn ground() -> Vec<String> {
    let mut lines = supporters();
    for item in ["redstone", "rail", "repeater", "comparator"] {
        lines.extend(reset());
        for x in FIRST..=LAST {
            lines.extend([format!("tp @s {x}.5 -26 0.5 180 0"), hold(item), "!place 0 0 2 up".into()]);
        }
        lines.push(String::new());
    }
    for block in ["redstone_wire", "rail", "unpowered_repeater", "unpowered_comparator"] {
        lines.extend(reset());
        lines.extend([format!("fill {FIRST} -25 2 {LAST} -25 2 {block}"), String::new(), format!("fill {FIRST} -25 3 {LAST} -25 3 stone"), format!("fill {FIRST} -25 3 {LAST} -25 3 air"), String::new()]);
    }
    lines.extend(reset());
    lines
}

fn sculk() -> Vec<String> {
    let mut lines = floor();
    for (block, z) in [("sculk_sensor", -2), ("sculk_shrieker", 0), ("calibrated_sculk_sensor", 2)] {
        lines.extend([hold(block), format!("!place 2 -1 {z} up"), String::new()]);
    }
    tidy(lines)
}

/// Carpet two over the floor by `/setblock`, by a one-cell `/fill` and by a three-cell `/fill`.
fn carpet() -> Vec<String> {
    let lines = with(floor(), &["setblock 2 -24 -4 white_carpet", "fill 4 -24 -4 4 -24 -4 white_carpet", "fill 6 -24 -4 8 -24 -4 white_carpet", "", "", ""]);
    tidy(lines)
}

/// Plates on stone, a fence and a top slab, their support then set to air one by one.
fn plate() -> Vec<String> {
    let mut lines = floor();
    let supports = ["stone", "oak_fence", "smooth_stone_slab [\"minecraft:vertical_half\"=\"top\"]"];
    let plates = ["stone_pressure_plate", "wooden_pressure_plate", "light_weighted_pressure_plate"];
    for (i, support) in supports.iter().enumerate() {
        for (j, plate) in plates.iter().enumerate() {
            let (x, z) = (2 + 2 * j as i32, -4 + 2 * i as i32);
            lines.extend([format!("setblock {x} -26 {z} {support}"), format!("setblock {x} -25 {z} {plate}")]);
        }
    }
    lines.push(String::new());
    for i in 0..supports.len() {
        for j in 0..plates.len() {
            lines.extend([format!("setblock {} -26 {} air", 2 + 2 * j, -4 + 2 * i as i32), String::new()]);
        }
    }
    tidy(lines)
}

/// Snow of each height on persistent leaves (z -2) and on stone (z 2); sand set five over each.
fn snow() -> Vec<String> {
    let mut lines = with(floor(), &["fill -1 -26 -2 7 -26 -2 oak_leaves [\"persistent_bit\"=true]", "fill -1 -26 2 7 -26 2 stone"]);
    for height in 0..8 {
        let x = height - 1;
        lines.extend([-2, 2].map(|z| format!("setblock {x} -25 {z} snow_layer [\"height\"={height}]")));
    }
    lines.extend([String::new(), "fill -1 -20 -2 7 -20 -2 sand".into(), "fill -1 -20 2 7 -20 2 sand".into()]);
    waits(&mut lines, 4);
    tidy(lines)
}

/// Six slightly and six very damaged anvils dropped eight, and six very damaged dropped three.
fn anvil() -> Vec<String> {
    let mut lines = floor();
    // Sapling refuses the older `anvil ["damage"=...]` that BDS takes.
    for (z, anvil, y) in [(-4, "chipped_anvil", -18), (0, "damaged_anvil", -18), (4, "damaged_anvil", -23)] {
        lines.push(format!("fill 2 {y} {z} 7 {y} {z} {anvil}"));
    }
    waits(&mut lines, 5);
    tidy(lines)
}

/// A bell put on the north face of a stone with another stone across a one-wide gap, on one with none,
/// and under a ceiling in a gap.
fn bell() -> Vec<String> {
    let mut lines = with(floor(), &["setblock 3 -26 -1 stone", "setblock 3 -26 1 stone", "setblock 3 -26 4 stone", "setblock 2 -24 -3 stone", "setblock 1 -25 -3 stone", "setblock 3 -25 -3 stone"]);
    lines.extend([hold("bell"), "!place 3 0 1 north".into(), String::new(), "!place 3 0 4 north".into(), String::new(), "!place 2 2 -3 down".into(), String::new()]);
    tidy(lines)
}

const VEHICLES: [&str; 7] = ["minecart", "chest_minecart", "hopper_minecart", "tnt_minecart", "command_block_minecart", "boat", "chest_boat"];

/// Each vehicle and a pig summoned with a name, then sought and killed by it.
fn names() -> Vec<String> {
    let mut lines = floor();
    for (x, kind) in (2..).step_by(2).zip(VEHICLES.iter().chain(&["pig"])) {
        lines.push(format!("summon {kind} Named{x} {x}.5 -26 4.5"));
    }
    lines.push(String::new());
    for x in (2..).step_by(2).take(VEHICLES.len() + 1) {
        lines.extend([format!("testfor @e[name=Named{x}]"), format!("kill @e[name=Named{x}]")]);
    }
    lines.extend(["kill @e[type=minecart]".into(), "kill @e[type=boat]".into()]);
    tidy(lines)
}

/// Minecarts summoned in and on a lava pool and a water pool.
fn lava() -> Vec<String> {
    let mut lines = with(floor(), &["fill 1 -29 -4 9 -27 4 stone", "fill 2 -28 -3 4 -27 3 lava", "fill 6 -28 -3 8 -27 3 water", "tp @s -3.5 -26 0.5 -90 0", ""]);
    lines.extend(owned(["summon minecart 3.5 -27 -2.5", "summon minecart 3.5 -26 2.5", "summon minecart 7.5 -27 -2.5", "summon minecart 7.5 -26 2.5"]));
    waits(&mut lines, 6);
    with(tidy(lines), &["fill 1 -29 -4 9 -28 4 air", "fill 1 -27 -4 9 -27 4 stone"])
}

fn pulse(lines: &mut Vec<String>, x: i32, y: i32, z: i32) {
    lines.extend([format!("setblock {x} {y} {z} redstone_block"), String::new(), format!("setblock {x} {y} {z} stone"), String::new()]);
}

/// Two dispensers shooting south at pots three away, one holding a diamond; then a pot shot from a
/// skeleton's range by a summoned arrow falling on it.
fn pot() -> Vec<String> {
    let mut lines = with(floor(), &["tp @s 0.5 -26 -1.5 180 0", "setblock 0 -26 -3 decorated_pot", "setblock 2 -26 -3 decorated_pot", hold("diamond 1").as_str(), "!click 0 0 -1 up", ""]);
    for x in [0, 2] {
        lines.extend([format!("setblock {x} -26 -6 dispenser [\"facing_direction\"=3]"), format!("replaceitem block {x} -26 -6 slot.container 0 arrow 1")]);
        pulse(&mut lines, x, -27, -6);
        lines.push(String::new());
    }
    lines.extend(owned(["setblock 4 -26 -3 decorated_pot", "summon arrow 4.5 -20 -2.5"]));
    waits(&mut lines, 4);
    tidy(lines)
}

/// r1disp's twelve arrows at a wall, on normal and then on hard.
fn dispensed() -> Vec<String> {
    let mut lines = with(floor(), &["fill -8 -26 8 16 -20 8 stone", "setblock 0 -26 -6 dispenser [\"facing_direction\"=3]", "tp @s -5.5 -26 0.5"]);
    for difficulty in ["normal", "hard"] {
        lines.extend([format!("difficulty {difficulty}"), "replaceitem block 0 -26 -6 slot.container 0 arrow 12".into(), String::new()]);
        for _ in 0..12 {
            pulse(&mut lines, 0, -27, -6);
        }
        lines.extend(["kill @e[type=arrow]".into(), String::new()]);
    }
    with(tidy(lines), &["difficulty easy"])
}

/// Old names set: `chain`, and `anvil` by each `damage`, alone and with a facing.
fn alias() -> Vec<String> {
    let mut lines = floor();
    let named = ["chain", "chain [\"pillar_axis\"=\"x\"]", "anvil", "anvil [\"damage\"=\"undamaged\"]", "anvil [\"damage\"=\"slightly_damaged\"]", "anvil [\"damage\"=\"very_damaged\"]", "anvil [\"damage\"=\"broken\"]", "anvil [\"damage\"=\"very_damaged\",\"minecraft:cardinal_direction\"=\"east\"]", "chipped_anvil [\"damage\"=\"very_damaged\"]"];
    for (x, block) in (2..).step_by(2).zip(named) {
        lines.extend([format!("setblock {x} -26 0 {block}"), format!("testforblock {x} -26 0 {block}")]);
    }
    lines.push(String::new());
    tidy(lines)
}

/// A lone copper chest and a pair put by hand, waxed and scraped sneaking; then unwaxed ones, a lone one
/// and a pair far apart, random-ticked fast.
fn copper() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", &hold("copper_chest 4"), "!place 2 -1 -3 up", "!place 2 -1 0 up", "!place 2 -1 1 up", ""]);
    lines.extend([hold("honeycomb 4"), "!sneak".into(), String::new()]);
    lines.extend(["!click 2 0 -3 up", "", "!click 2 0 0 up", ""].map(String::from));
    lines.extend([hold("iron_axe"), "!click 2 0 -3 up".into(), String::new(), "!click 2 0 1 up".into(), String::new(), "!unsneak".into(), "gamemode creative @s".into()]);
    lines.extend([hold("copper_chest 4"), "tp @s 8.5 -26 -3.5 -90 0".into(), "!place 2 -1 -2 up".into(), "tp @s 8.5 -26 4.5 -90 0".into(), "!place 2 -1 0 up".into(), "!place 2 -1 1 up".into(), "gamerule randomtickspeed 4096".into()]);
    waits(&mut lines, 15);
    with(tidy(lines), &["gamerule randomtickspeed 1"])
}
