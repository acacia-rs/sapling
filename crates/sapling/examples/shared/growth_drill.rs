//! Drills of random ticks by light and by crop (docs/research/growth.md "Light"): sealed stone rooms
//! lit by light blocks to a set level, and four crops side by side. `randomtickspeed` 200, as `grows`.

use super::creatures::{floor, waits, with};

/// Packets shown of these drills.
pub const GROWTH_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "LevelEvent", "LevelSoundEvent", "AddItemEntity"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "lightgrow" => light_grow(),
            "croprates" => crop_rates(),
            "lightpoke" => light_poke(),
            _ => return None,
        },
        GROWTH_SHOWN,
    ))
}

const WET: &str = "farmland [\"moisturized_amount\"=7]";
/// Sapling's drill world has ground at y 60 to 63 over the drills (drill_server.rs), where BDS's is
/// open sky: a hole in it lets the daylight down; on BDS it is a fill of nothing.
const SKY: &str = "fill -8 60 -8 16 63 8 air";

/// What a room holds, as (x offset in the room: 0 or 1, y, block); the room's floor is y -27,
/// its air -26 to -24.
type Room = &'static [(i32, i32, &'static str)];

/// A crop or stem on wet farmland by water, lit to `light` - 1 in its cell from beside it.
macro_rules! crop {
    ($plant:literal, $light:literal) => {
        &[(0, -27, WET), (0, -26, $plant), (1, -27, "water"), (1, -26, concat!("light_block_", $light))]
    };
}

/// A soil spreading to dirt beside it, lit to `light` - 2 over itself.
macro_rules! soil {
    ($soil:literal, $light:literal) => {
        &[(0, -27, $soil), (1, -27, "dirt"), (0, -25, concat!("light_block_", $light))]
    };
}

/// A block in the room's first cell lit to `light` - 1 from beside it.
macro_rules! lit {
    ($block:literal, $light:literal) => {
        &[(0, -26, $block), (1, -26, concat!("light_block_", $light))]
    };
}

/// Four rows of seven rooms, row z -6 to 6 by 4, room x -6 to 12 by 3.
const ROOMS: [[Room; 7]; 4] = [
    [crop!("wheat", 10), crop!("wheat", 9), crop!("wheat", 8), &[(0, -27, WET), (0, -26, "wheat"), (1, -27, "water")], crop!("melon_stem", 10), crop!("melon_stem", 9), crop!("carrots", 10)],
    [
        &[(0, -27, "dirt"), (0, -26, "bamboo"), (1, -25, "light_block_10")],
        &[(0, -27, "dirt"), (0, -26, "bamboo"), (1, -25, "light_block_9")],
        soil!("grass_block", 11),
        soil!("grass_block", 10),
        soil!("mycelium", 11),
        soil!("mycelium", 10),
        &[(0, -27, "grass_block"), (1, -27, "grass_block"), (1, -26, "stone")],
    ],
    [
        &[(0, -27, "mycelium"), (1, -27, "mycelium"), (1, -26, "stone")],
        lit!("ice", 13),
        lit!("ice", 12),
        lit!("snow_layer [\"height\"=7]", 14),
        lit!("snow_layer [\"height\"=7]", 13),
        lit!("snow_layer [\"height\"=7]", 12),
        lit!("brown_mushroom", 13),
    ],
    [lit!("brown_mushroom", 14), &[(0, -27, "mycelium"), (0, -26, "brown_mushroom"), (1, -26, "light_block_15")], &[(0, -26, "brown_mushroom")], crop!("wheat", 10), crop!("wheat", 9), crop!("wheat", 8), crop!("melon_stem", 9)],
];

/// The rooms of `ROOMS` dug out of a stone block and filled, the bots on its top, and a minute of
/// random ticks at 200.
pub fn light_grow() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "fill -7 -28 -7 15 -23 7 stone", "tp @s 0.5 -22 0.5", "tp CmdWatcher 0.5 -22 0.5", ""]);
    for (row, rooms) in ROOMS.iter().enumerate() {
        let z = -6 + 4 * row as i32;
        for (k, room) in rooms.iter().enumerate() {
            let x = -6 + 3 * k as i32;
            lines.push(format!("fill {x} -26 {z} {} -24 {z} air", x + 1));
            lines.extend(room.iter().map(|(dx, y, block)| format!("setblock {} {y} {z} {block}", x + dx)));
        }
    }
    lines.extend(["", "gamerule randomtickspeed 200"].map(String::from));
    waits(&mut lines, 60);
    with(lines, &["gamerule randomtickspeed 1", "fill -7 -28 -7 15 -23 7 air", "fill -8 -27 -8 16 -27 8 stone", "kill @e[type=item]"])
}

/// Whether light is asked when a neighbour changes: mushrooms in daylight on stone and on
/// mycelium, and wheat shut in stone, each poked from the side and the wheat from below too.
pub fn light_poke() -> Vec<String> {
    let mut lines = with(floor(), &[
        "time set noon",
        SKY,
        "fill -8 -28 -8 16 -28 8 stone",
        "fill -6 -27 -6 -2 -24 -2 stone",
        format!("setblock -4 -27 -4 {WET}").as_str(),
        "setblock -4 -26 -4 wheat",
        "setblock 6 -26 0 brown_mushroom",
        "setblock 9 -27 0 mycelium",
        "setblock 9 -26 0 brown_mushroom",
        "",
        "",
    ]);
    lines.extend(["setblock 7 -26 0 stone", "setblock 10 -26 0 stone", "setblock -5 -26 -4 dirt", "", "", "setblock -4 -27 -4 farmland [\"moisturized_amount\"=6]", "", "", ""].map(String::from));
    with(lines, &["fill -6 -27 -6 -2 -24 -2 air", "fill -8 -26 -8 16 -26 8 air", "fill -8 -27 -8 16 -27 8 stone", "kill @e[type=item]"])
}

/// Wheat, beetroot, carrots and potatoes in four 5 by 5 fields of wet farmland round a water
/// cell, at noon, a minute of random ticks at 200.
pub fn crop_rates() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", SKY, "fill -8 -28 -8 16 -28 8 stone"]);
    for (crop, x, z) in [("wheat", 2, -7), ("beetroot", 9, -7), ("carrots", 2, 2), ("potatoes", 9, 2)] {
        lines.push(format!("fill {x} -27 {z} {} -27 {} {WET}", x + 4, z + 4));
        lines.push(format!("setblock {} -27 {} water", x + 2, z + 2));
        lines.push(format!("fill {x} -26 {z} {} -26 {} {crop} replace air", x + 4, z + 4));
        lines.push(format!("setblock {} -26 {} air", x + 2, z + 2));
    }
    lines.extend(["", "gamerule randomtickspeed 200"].map(String::from));
    waits(&mut lines, 60);
    with(lines, &["gamerule randomtickspeed 1", "fill -8 -26 -8 16 -26 8 air", "fill -8 -27 -8 16 -27 8 stone", "kill @e[type=item]"])
}
