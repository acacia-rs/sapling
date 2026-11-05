//! The thirteenth wave's group L drills (research/parity/THIRTEENTH-BRIEF.md): the random tick's pace at a
//! rule high enough that a cell comes up more than once a tick (`l2pace`).

use super::creatures::{floor, waits};

const SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "CommandOutput"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "l2pace" => (pace(), SHOWN),
        "l2farm" => (farmland(), SHOWN),
        "l2carts" => (carts(), CARTS_SHOWN),
        "l2shroom" => (mushrooms(), SHOWN),
        "l2meal" => (crop_meal(), SHOWN),
        _ => return None,
    })
}

/// In survival, wheat set at growth 0 a block east of the actor on wet farmland and bone-mealed once,
/// 30 times over: how far one click grows a crop.
fn crop_meal() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamerule randomtickspeed 0", "fill ~-1 ~-1 ~7 ~1 ~-1 ~9 stone", "tp @s ~ ~ ~8 -90 0", "fill ~1 ~ ~-3 ~7 ~8 ~3 air", "fill ~1 ~-1 ~-3 ~7 ~-1 ~3 farmland [\"moisturized_amount\"=7]", "gamemode survival @s", "clear @s", "replaceitem entity @s slot.weapon.mainhand 0 bone_meal 64", ""]
        .map(String::from)
        .into();
    for _ in 0..30 {
        lines.extend(["setblock ~1 ~ ~ wheat", "!click 1 0 0", ""].map(String::from));
    }
    lines.extend(["setblock ~1 ~ ~ air", "clear @s", "gamemode creative @s", "gamerule randomtickspeed 1", ""].map(String::from));
    lines
}

/// Twelve shut stone rooms 10 apart (5 by 2 by 3 of air), a brown mushroom in each, at `randomtickspeed` 200
/// for 60 s: how soon and how often each spreads (the plants drill's one room spread 3 times).
fn mushrooms() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["fill 0 -27 10 36 -27 37 stone", "fill 0 -26 10 36 -22 37 air", ""].map(String::from));
    for (x, z) in [0, 10, 20, 30].into_iter().flat_map(|x| [10, 20, 30].map(move |z| (x, z))) {
        lines.extend([format!("fill {x} -26 {z} {} -23 {} stone", x + 6, z + 3), format!("fill {} -26 {} {} -24 {} air", x + 1, z + 1, x + 5, z + 2), format!("setblock {} -26 {} brown_mushroom", x + 3, z + 1)]);
    }
    lines.extend(["gamerule randomtickspeed 200", ""].map(String::from));
    waits(&mut lines, 60);
    lines.extend(["gamerule randomtickspeed 0", "fill 0 -26 10 36 -22 37 air", ""].map(String::from));
    lines
}

const CARTS_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion"];

/// Rails east along z -2, -4, -6 and -8 from x 2: at z -2 a cart summoned mid-slope (rising west to a
/// stone); at z -4 one sped east by three powered rails into a slope rising east; at z -6 two summoned
/// 0.4 apart; at z -8 one sped east into one at rest. The carts are left in the world for its folder.
fn carts() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["setblock 1 -26 -2 stone", "setblock 2 -26 -2 rail [\"rail_direction\"=3]", "fill 3 -26 -2 8 -26 -2 rail [\"rail_direction\"=1]"].map(String::from));
    for z in [-4, -8] {
        lines.extend([format!("setblock 1 -26 {z} stone"), format!("fill 2 -27 {z} 4 -27 {z} redstone_block"), format!("fill 2 -26 {z} 4 -26 {z} golden_rail [\"rail_direction\"=1]"), format!("fill 5 -26 {z} 11 -26 {z} rail [\"rail_direction\"=1]")]);
    }
    lines.extend(["setblock 12 -26 -4 rail [\"rail_direction\"=2]", "setblock 13 -26 -4 stone", "fill 12 -26 -8 14 -26 -8 rail [\"rail_direction\"=1]", "fill 2 -26 -6 12 -26 -6 rail [\"rail_direction\"=1]", ""].map(String::from));
    lines.extend(["summon minecart 2.5 -26 -1.5", "summon minecart 4.5 -26 -5.5", "summon minecart 4.9 -26 -5.5", "summon minecart 10.5 -26 -7.5", ""].map(String::from));
    lines.extend(["summon minecart 2.5 -26 -3.5", "summon minecart 2.5 -26 -7.5"].map(String::from));
    waits(&mut lines, 20);
    lines
}

/// Farmland with no water near, at `randomtickspeed` 20 for 40 s then 200 for 10 s: 64 dry cells in chunk 0,0
/// (each turns to dirt at its first random tick) and 64 wet ones under wheat in chunk 0,-1 (a step drier
/// each random tick, kept by the crop), both chunks' highest block in section 2.
fn farmland() -> Vec<String> {
    let mut lines = floor();
    for rule in [20, 200] {
        lines.extend(["fill 1 -27 1 8 -27 8 farmland [\"moisturized_amount\"=0]", "fill 1 -27 -8 8 -27 -1 farmland [\"moisturized_amount\"=7]", "fill 1 -26 -8 8 -26 -1 wheat", ""].map(String::from));
        lines.push(format!("gamerule randomtickspeed {rule}"));
        waits(&mut lines, if rule == 20 { 40 } else { 10 });
        lines.extend(["gamerule randomtickspeed 0", "fill 1 -26 -8 8 -26 8 air", ""].map(String::from));
    }
    lines.push("gamerule randomtickspeed 1".to_string());
    lines
}

/// 91 stalactites hanging from dripstone blocks under a water pool in chunk 0,0 (its highest block in
/// section 2): three rounds of 6 s at `randomtickspeed` 4096, then one of 20 s at 256. Each root's
/// first growth (a piece under y -21) times its random ticks: 0.0114 of them grow.
fn pace() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["fill 0 -20 0 14 -18 8 stone", "fill 1 -19 1 13 -19 7 water", "fill 1 -20 1 13 -20 7 dripstone_block", ""].map(String::from));
    for rule in [4096, 4096, 4096, 256] {
        lines.extend(["fill 1 -26 1 13 -21 7 air", "fill 1 -21 1 13 -21 7 pointed_dripstone [\"dripstone_thickness\"=\"tip\",\"hanging\"=true]", ""].map(String::from));
        lines.push(format!("gamerule randomtickspeed {rule}"));
        waits(&mut lines, if rule > 256 { 6 } else { 20 });
        lines.extend(["gamerule randomtickspeed 0", ""].map(String::from));
    }
    lines.push("gamerule randomtickspeed 1".to_string());
    lines
}
