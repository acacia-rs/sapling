//! The thirteenth wave's group I drills (research/parity/THIRTEENTH-BRIEF.md): a detector rail met by a
//! rolling minecart (`i2rails`) and how far a hopper minecart reaches for items (`i2reach`).

use super::creatures::{floor, waits};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "i2rails" => (rails(), RAILS_SHOWN),
        "i2reach" => (reach(), RAILS_SHOWN),
        "i2reach2" => (reach_edges(), RAILS_SHOWN),
        "i2drip" => (dripstone(), DRIP_SHOWN),
        "i2grow" => (growth(), &["UpdateBlock", "UpdateSubchunkBlocks", "CommandOutput"]),
        _ => return None,
    })
}

const RAILS_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "CommandOutput", "AddEntity", "AddItemEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute"];

/// A minecart let go on a slope at x 0 (risen to a stone at x -1) at z -4, and one on a powered rail
/// at x 0 against a stone at z -6, each with rails east over a detector rail at x 5 to a stone at x
/// 12: the press is timed against the moves that bring the cart over the rail's middle, the let-go
/// against the press. A third cart stands on a detector rail at z 2 for 25 s, to time the
/// re-checks while it stays.
fn rails() -> Vec<String> {
    let mut lines = floor();
    lines.extend([
        "setblock -1 -26 -4 stone",
        "setblock 0 -26 -4 rail [\"rail_direction\"=3]",
        "fill 1 -26 -4 11 -26 -4 rail [\"rail_direction\"=1]",
        "setblock 5 -26 -4 detector_rail [\"rail_direction\"=1]",
        "setblock 12 -26 -4 stone",
        "setblock 2 -26 2 detector_rail",
        "setblock -1 -26 -6 stone",
        "setblock 0 -27 -6 redstone_block",
        "setblock 0 -26 -6 golden_rail [\"rail_direction\"=1]",
        "fill 1 -26 -6 11 -26 -6 rail [\"rail_direction\"=1]",
        "setblock 5 -26 -6 detector_rail [\"rail_direction\"=1]",
        "setblock 12 -26 -6 stone",
    ].map(String::from));
    waits(&mut lines, 1);
    lines.push("summon minecart 0.5 -26 -3.5".to_string());
    waits(&mut lines, 3);
    lines.push("summon minecart 0.5 -26 -5.5".to_string());
    waits(&mut lines, 6);
    lines.push("summon minecart 2.5 -26 2.5".to_string());
    waits(&mut lines, 25);
    lines.push("kill @e[type=minecart]".to_string());
    waits(&mut lines, 3);
    lines
}

/// Where items are dropped before each is moved next to a cart.
const HOLD: &str = "14.5 -26 -7.5";

/// A dirt item dropped at [`HOLD`] and moved to `x y z` (it flies off a little as it drops).
fn item_to(x: f32, y: f32, z: f32) -> [String; 2] {
    [format!("loot spawn {HOLD} mine 8 -26 -8"), format!("tp @e[type=item,x=14,y=-26,z=-8,r=4] {x} {y} {z}")]
}

/// Hopper minecarts at x 0.5 with items on the floor 0.85 to 0.95 east, and at x 6.5 with items
/// 0.45 east on a carpet, a daylight detector and a slab over two blocks: where its reach ends.
fn reach_edges() -> Vec<String> {
    let mut lines = floor();
    lines.push("setblock 8 -26 -8 dirt".to_string());
    let east: [f32; 3] = [1.35, 1.38, 1.45];
    let tops = [(-23.9375, "white_carpet"), (-23.625, "daylight_detector"), (-23.5, "oak_slab")];
    for (i, _) in east.iter().enumerate() {
        lines.push(format!("setblock 0 -26 {} rail", -6 + 3 * i as i32));
    }
    for (i, (_, on)) in tops.iter().enumerate() {
        let z = -6 + 3 * i as i32;
        lines.extend([format!("setblock 6 -26 {z} rail"), format!("fill 7 -26 {z} 7 -25 {z} stone"), format!("setblock 7 -24 {z} {on}")]);
    }
    waits(&mut lines, 1);
    let carts = (0..3).map(|i| (0.5, -5.5 + 3.0 * i as f32)).chain((0..3).map(|i| (6.5, -5.5 + 3.0 * i as f32)));
    lines.extend(carts.map(|(x, z)| format!("summon hopper_minecart {x} -26 {z}")));
    waits(&mut lines, 1);
    for (i, x) in east.iter().enumerate() {
        lines.extend(item_to(*x, -26.0, -5.5 + 3.0 * i as f32));
    }
    for (i, (top, _)) in tops.iter().enumerate() {
        lines.extend(item_to(6.95, *top, -5.5 + 3.0 * i as f32));
    }
    waits(&mut lines, 2);
    lines.push("kill @e[type=hopper_minecart]".to_string());
    lines
}

const DRIP_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "UpdateBlockSynced", "CommandOutput", "AddEntity", "AddItemEntity", "RemoveEntity", "LevelEvent", "EntityEvent", "UpdateAttributes", "MoveEntityDelta"];

/// Pointed dripstone standing up: a two-piece stalagmite whose stone is taken away, then the actor
/// in survival dropped six blocks onto a stalagmite's tip and, for the same fall, onto stone.
fn dripstone() -> Vec<String> {
    let piece = |thickness: &str| format!("pointed_dripstone [\"dripstone_thickness\"=\"{thickness}\",\"hanging\"=false]");
    let mut lines = floor();
    lines.extend([
        "gamerule falldamage true".to_string(),
        "setblock 2 -26 4 stone".to_string(),
        format!("setblock 2 -25 4 {}", piece("frustum")),
        format!("setblock 2 -24 4 {}", piece("tip")),
        format!("setblock 6 -26 6 {}", piece("tip")),
    ]);
    waits(&mut lines, 1);
    lines.push("setblock 2 -26 4 air".to_string());
    waits(&mut lines, 2);
    lines.extend(["gamemode survival @s", "tp @s 6.5 -20 6.5"].map(String::from));
    waits(&mut lines, 3);
    lines.extend(["effect @s instant_health 1 10 true", "tp @s 10.5 -20 6.5"].map(String::from));
    waits(&mut lines, 3);
    lines.push("gamemode creative @s".to_string());
    lines
}

/// Stalactites from dripstone blocks under water at a high `randomtickspeed`: one with five blocks
/// of air under its tip, one over a stalagmite's tip two below it, one under stone (which never grows).
fn growth() -> Vec<String> {
    let mut lines = floor();
    for (x, under) in [(2, None), (6, Some("tip")), (10, None)] {
        let top = if x == 10 { "stone" } else { "dripstone_block" };
        // The water sits in a stone basin, so none runs down past the stalactite.
        lines.extend([format!("fill {} -20 3 {} -19 5 stone", x - 1, x + 1), format!("setblock {x} -19 4 water"), format!("setblock {x} -20 4 {top}")]);
        lines.push(format!("setblock {x} -21 4 pointed_dripstone [\"dripstone_thickness\"=\"tip\",\"hanging\"=true]"));
        if let Some(thickness) = under {
            lines.extend([format!("fill {x} -26 4 {x} -25 4 stone"), format!("setblock {x} -24 4 pointed_dripstone [\"dripstone_thickness\"=\"{thickness}\",\"hanging\"=false]")]);
        }
    }
    lines.push("gamerule randomtickspeed 4096".to_string());
    waits(&mut lines, 40);
    lines.push("gamerule randomtickspeed 1".to_string());
    lines
}

/// Hopper minecarts on rails, each given one item moved next to it: at x 0.5, items on the floor
/// 0.55 to 0.8 east; at x 6.5, items 0.45 east on ledges from half a block to four blocks up; at x 12.5
/// on a raised rail, one item on the floor below and one on a ledge level with it.
fn reach() -> Vec<String> {
    let mut lines = floor();
    lines.push("setblock 8 -26 -8 dirt".to_string());
    let east: [f32; 7] = [1.05, 1.1, 1.12, 1.13, 1.15, 1.2, 1.3];
    let tops: [f32; 8] = [-25.5, -25.0,-24.5, -24.0, -23.5, -23.0, -22.5, -22.0];
    let row = |i: usize| -7.5 + 2.0 * i as f32;
    for (i, _) in east.iter().enumerate() {
        lines.push(format!("setblock 0 -26 {} rail", row(i).floor()));
    }
    for (i, top) in tops.iter().enumerate() {
        let z = row(i).floor();
        lines.push(format!("setblock 6 -26 {z} rail"));
        let whole = top.floor() as i32;
        if whole > -26 {
            lines.push(format!("fill 7 -26 {z} 7 {} {z} stone", whole - 1));
        }
        if top.fract() != 0.0 {
            lines.push(format!("setblock 7 {whole} {z} oak_slab"));
        }
    }
    lines.extend(["setblock 12 -26 -8 stone", "setblock 12 -25 -8 rail", "setblock 12 -26 -6 stone", "setblock 12 -25 -6 rail", "setblock 13 -26 -6 stone"].map(String::from));
    waits(&mut lines, 1);
    let carts = (0..east.len()).map(|i| (0.5, -26.0, row(i))).chain((0..tops.len()).map(|i| (6.5, -26.0, row(i)))).chain([(12.5, -25.0, -7.5), (12.5, -25.0, -5.5)]);
    lines.extend(carts.map(|(x, y, z)| format!("summon hopper_minecart {x} {y} {z}")));
    waits(&mut lines, 1);
    let items = east.iter().enumerate().map(|(i, x)| (*x, -26.0, row(i))).chain(tops.iter().enumerate().map(|(i, top)| (6.95, *top, row(i)))).chain([(12.95, -26.0, -7.5), (12.95, -25.0, -5.5)]);
    for (x, y, z) in items {
        lines.extend(item_to(x, y, z));
    }
    waits(&mut lines, 2);
    lines.push("kill @e[type=hopper_minecart]".to_string());
    lines
}
