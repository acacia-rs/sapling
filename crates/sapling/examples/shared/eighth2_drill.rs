//! Drills of the eighth wave's group 2: lava's random tick lighting fire and which blocks lava takes
//! (docs/research/fire.md, assumptions-checked.md 4), what stood on a burnt block, who lights redstone
//! ore by standing on it and lit ore's pace (block-uses.md "Redstone ore").

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "AddEntity", "RemoveEntity", "AddItemActor"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "lavafire" => (lava_fire(), SHOWN),
        "lavaeats" => (lava_eats(), SHOWN),
        "burnfloat" => (burn_float(), SHOWN),
        "orestand" => (ore_stand(), SHOWN),
        "orepace" => (ore_pace(), SHOWN),
        _ => return None,
    })
}

/// Pockets' x on the bots' platform; each spans x - 1 to x + 2.
const POCKETS: [i32; 5] = [-6, -1, 4, 9, 14];

/// Five lava pockets, twice (rows z -6 and 4), on a floor doubled at y -28, at `randomTickSpeed` 40 for
/// 30 s; each pocket's first fire says which rule lit it. Lava in a hole in the floor at (x, -27, z):
/// a plank beside and up (x + 1, -26, z + 1); a plank over the lava; lava on the floor in a ring with a
/// plank in the floor beside and down; the lava covered by stone, the plank beside and up; and the first
/// with stone on the plank's +x side.
fn lava_fire() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule randomtickspeed 0", "gamerule dofiretick true", "time set noon", "fill -8 -28 -8 16 -28 8 stone"]);
    for z in [-6, 4] {
        let [a, b, c, d, e] = POCKETS;
        lines.extend([
            format!("setblock {a} -27 {z} lava"),
            format!("setblock {} -26 {} oak_planks", a + 1, z + 1),
            format!("setblock {b} -27 {z} lava"),
            format!("setblock {b} -26 {z} oak_planks"),
            format!("fill {} -26 {} {} -26 {} stone", c - 1, z - 1, c + 1, z + 1),
            format!("setblock {c} -26 {z} lava"),
            format!("setblock {} -26 {} air", c + 1, z + 1),
            format!("setblock {} -27 {} oak_planks", c + 1, z + 1),
            format!("setblock {d} -27 {z} lava"),
            format!("setblock {d} -26 {z} stone"),
            format!("setblock {} -26 {} oak_planks", d + 1, z + 1),
            format!("setblock {e} -27 {z} lava"),
            format!("setblock {} -26 {} oak_planks", e + 1, z + 1),
            format!("setblock {} -26 {} stone", e + 2, z + 1),
        ]);
    }
    lines.push("gamerule randomtickspeed 40".into());
    waits(&mut lines, 30);
    lines.extend(["gamerule randomtickspeed 0", "fill -8 -28 -8 16 -28 8 air", "fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -20 8 air"].map(String::from));
    lines.extend(["gamerule randomtickspeed 1", ""].map(String::from));
    lines
}

/// Blocks lava may take, one a cell at even x from -6 in three rows: a row of lava sources at z, each
/// candidate at z + 1 walled in stone, a stone lid over all; the lids over the lava are taken off at once.
const EATEN: [[&str; 12]; 3] = [
    [
        "stone_button [\"facing_direction\"=1]", "stone_pressure_plate", "standing_sign", "standing_banner", "white_carpet", "moss_carpet", "unpowered_repeater",
        "skeleton_skull [\"facing_direction\"=1]", "flower_pot", "end_rod [\"facing_direction\"=1]", "sea_pickle", "decorated_pot",
    ],
    [
        "conduit", "snow_layer", "candle", "torch [\"torch_facing_direction\"=\"top\"]", "rail", "web", "short_grass", "lever [\"lever_direction\"=\"up_north_south\"]",
        "redstone_wire", "ladder [\"facing_direction\"=2]", "oak_fence", "glass_pane",
    ],
    [
        "iron_bars", "cake", "leaf_litter", "pink_petals", "heavy_core", "pointed_dripstone", "small_amethyst_bud", "lantern", "lightning_rod", "wooden_pressure_plate",
        "trapdoor", "brewing_stand",
    ],
];
const ROWS: [i32; 3] = [-7, -2, 3];

fn lava_eats() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule randomtickspeed 0", "tp @s 0.5 -26 7.5", "tp CmdWatcher 2.5 -26 7.5"]);
    for (row, z) in EATEN.iter().zip(ROWS) {
        lines.extend([format!("fill -7 -26 {} 17 -25 {} stone", z - 1, z + 2), format!("fill -6 -26 {z} 16 -26 {z} lava")]);
        for (i, block) in row.iter().enumerate() {
            lines.push(format!("setblock {} -26 {} {block}", -6 + 2 * i as i32, z + 1));
        }
    }
    waits(&mut lines, 3);
    for z in ROWS {
        lines.push(format!("fill -6 -25 {z} 16 -25 {z} air"));
    }
    waits(&mut lines, 6);
    for z in ROWS {
        lines.push(format!("fill -7 -26 {} 17 -25 {} air", z - 1, z + 2));
    }
    lines.extend(["kill @e[type=item]", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", "gamerule randomtickspeed 1", ""].map(String::from));
    lines
}

/// Lone planks at z -6 with fire beside each (x + 1) and on each a torch, sand, a rail, dust, a stone
/// plate, a lantern and a candle, at `randomTickSpeed` 40 for 25 s; then what is over each plank's cell.
const ON_PLANKS: [&str; 7] = ["torch [\"torch_facing_direction\"=\"top\"]", "sand", "rail", "redstone_wire", "stone_pressure_plate", "lantern", "candle"];

fn burn_float() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule randomtickspeed 0", "gamerule dofiretick true", "time set noon"]);
    let xs: Vec<i32> = (0..ON_PLANKS.len() as i32).map(|i| -6 + 3 * i).collect();
    for (x, top) in xs.iter().zip(ON_PLANKS) {
        lines.extend([format!("setblock {x} -26 -6 oak_planks"), format!("setblock {x} -25 -6 {top}")]);
    }
    for x in &xs {
        lines.push(format!("setblock {} -26 -6 fire", x + 1));
    }
    lines.push("gamerule randomtickspeed 40".into());
    waits(&mut lines, 25);
    lines.push("gamerule randomtickspeed 0".into());
    for x in &xs {
        lines.extend([format!("testforblock {x} -26 -6 air"), format!("testforblock {x} -25 -6 air")]);
    }
    lines.extend(["fill -8 -26 -8 16 -20 8 air", "kill @e[type=item]", "kill @e[type=falling_block]", "gamerule randomtickspeed 1", ""].map(String::from));
    lines
}

/// Redstone ore set under the creative actor, then under it in survival; under a held cow, a held
/// zombie (fire resistant, at noon) and a held armour stand; each read two seconds on.
fn ore_stand() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule randomtickspeed 0", "time set noon", "tp @s 4.5 -26 4.5", "tp CmdWatcher 0.5 -26 -4.5"]);
    waits(&mut lines, 2);
    lines.push("setblock 4 -27 4 redstone_ore".into());
    waits(&mut lines, 2);
    lines.extend(["testforblock 4 -27 4 lit_redstone_ore", "setblock 4 -27 4 stone", "gamemode survival @s", "", "setblock 4 -27 4 redstone_ore"].map(String::from));
    waits(&mut lines, 2);
    lines.extend(["testforblock 4 -27 4 lit_redstone_ore", "gamemode creative @s", "setblock 4 -27 4 stone"].map(String::from));
    let mobs = [("cow", 8), ("zombie", 11), ("armor_stand", 14)];
    for (kind, x) in mobs {
        lines.push(format!("summon {kind} {x}.5 -26 -2.5"));
    }
    lines.extend(["effect @e[type=!player] slowness 60 255 true", "effect @e[type=zombie] fire_resistance 60 0 true"].map(String::from));
    waits(&mut lines, 3);
    for (_, x) in mobs {
        lines.push(format!("setblock {x} -27 -3 redstone_ore"));
    }
    waits(&mut lines, 3);
    for (_, x) in mobs {
        lines.push(format!("testforblock {x} -27 -3 lit_redstone_ore"));
    }
    lines.extend(["kill @e[type=!player]", "fill -8 -27 -8 16 -27 8 stone", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", "gamerule randomtickspeed 1", ""].map(String::from));
    lines
}

/// Twelve lit redstone ores in the floor (z 3) at `randomTickSpeed` 1 for 60 s, three times: each
/// round's count out is firepace's first row (fire.md, sapling's 1 and 10 of 12 against BDS's 6).
fn ore_pace() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule randomtickspeed 0", "time set noon"]);
    for _ in 0..3 {
        lines.extend(["fill -6 -27 3 5 -27 3 lit_redstone_ore", "gamerule randomtickspeed 1"].map(String::from));
        waits(&mut lines, 60);
        lines.extend(["gamerule randomtickspeed 0", "fill -6 -27 3 5 -27 3 stone"].map(String::from));
    }
    lines.extend(["gamerule randomtickspeed 1", ""].map(String::from));
    lines
}
