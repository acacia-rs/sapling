//! The thirteenth wave's group P drills (research/parity/THIRTEENTH-BRIEF.md): carts parting and climbing
//! into a block (`p2carts`), where bone meal brings up plants around grass and a poppy (`p2meal`),
//! placing onto soils and supports (`p2place`), and the blocks and redstone lines of REMAINING3 in
//! thirteenth2p_parts.rs and thirteenth2p_uses.rs.

#[path = "thirteenth2p_parts.rs"]
mod parts;
#[path = "thirteenth2p_uses.rs"]
mod uses;
#[path = "thirteenth2p_more.rs"]
mod more;

use super::creatures::{floor, waits};

const SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "CommandOutput", "LevelEvent"];
const CARTS_SHOWN: &[&str] = &["CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "UpdateBlock", "UpdateSubchunkBlocks"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "p2carts" => (carts(), CARTS_SHOWN),
        "p2meal" => (meal(), SHOWN),
        "p2place" => (placing(), SHOWN),
        _ => return parts::drill(name).or_else(|| uses::drill(name)).or_else(|| more::drill(name)),
    })
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// Rails east along z -2, -4, -6 and -8 from x 2 to 14, a cart at rest at x 4.5 on each and a second one
/// summoned 0.2, 0.5, 0.7 and 0.9 east of it; at z 3 a cart sped east by three powered rails up a slope
/// at x 12, joined to a rail on stone at 13 that then turns to stone.
fn carts() -> Vec<String> {
    let mut lines = floor();
    for z in [-2, -4, -6, -8] {
        lines.push(format!("fill 2 -26 {z} 14 -26 {z} rail [\"rail_direction\"=1]"));
    }
    lines.extend(owned(["setblock 1 -26 3 stone", "fill 2 -27 3 4 -27 3 redstone_block", "fill 2 -26 3 4 -26 3 golden_rail [\"rail_direction\"=1]"]));
    lines.extend(owned(["fill 5 -26 3 11 -26 3 rail [\"rail_direction\"=1]", "setblock 13 -26 3 stone", "setblock 13 -25 3 rail [\"rail_direction\"=1]", "setblock 12 -26 3 rail [\"rail_direction\"=2]", ""]));
    lines.extend(owned(["setblock 13 -25 3 stone", ""]));
    for z in [-2, -4, -6, -8] {
        lines.push(format!("summon minecart 4.5 -26 {}", z as f32 + 0.5));
    }
    lines.push(String::new());
    for (z, apart) in [(-2, 0.2), (-4, 0.5), (-6, 0.7), (-8, 0.9)] {
        lines.push(format!("summon minecart {} -26 {}", 4.5 + apart, z as f32 + 0.5));
    }
    lines.push("summon minecart 2.5 -26 3.5".to_owned());
    waits(&mut lines, 15);
    lines.extend(owned(["kill @e[type=minecart]", "fill 1 -26 -8 14 -25 3 air", "fill 2 -27 3 4 -27 3 stone", ""]));
    lines
}

/// Creative bone meal, ten clicks each: on the middle of a field of grass (x -2 to 4, z -6 to 0), then on a
/// poppy in the middle of another (z 1 to 7); the plants are cleared after each click.
fn meal() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["fill -2 -27 -6 4 -27 7 grass", "replaceitem entity @s slot.weapon.mainhand 0 bone_meal 64", ""]));
    for _ in 0..10 {
        lines.extend(owned(["!click 1 -1 -3", "", "fill -2 -26 -6 4 -24 0 air", ""]));
    }
    for _ in 0..10 {
        lines.extend(owned(["setblock 1 -26 4 poppy", "", "!click 1 0 4", "", "fill -2 -26 1 4 -24 7 air", ""]));
    }
    lines.extend(owned(["fill -2 -27 -6 4 -27 7 stone", "clear @s", ""]));
    lines
}

/// The actor places: a rail, wire and a repeater on a fence, glass, a top slab and a hopper set into the
/// floor (x 2, sneaking for the hopper); fences beside leaves, glass, stairs, a slab and a chest, and a wall
/// beside a fence (x 3, neighbours at x 4); glow berries under top slabs and stone, eight times; weeping
/// vines under stone and twisting vines on it; a shulker box and an end rod on three faces; stone onto snow
/// one and two layers deep; kelp on stone in a pit of water whose floor turns to magma, and a coral fan on
/// the pit's sand.
fn placing() -> Vec<String> {
    const SUPPORTS: [&str; 4] = ["oak_fence", "glass", "smooth_stone_slab [\"minecraft:vertical_half\"=\"top\"]", "hopper"];
    const BESIDE: [&str; 7] = ["oak_leaves", "glass", "oak_stairs [\"weirdo_direction\"=0]", "oak_stairs [\"weirdo_direction\"=1]", "smooth_stone_slab", "chest", "oak_fence"];
    let mut lines = floor();
    for item in ["rail", "redstone", "repeater"] {
        for (support, z) in SUPPORTS.iter().zip(-2..) {
            lines.push(format!("setblock 2 -27 {z} {support}"));
        }
        lines.push(format!("replaceitem entity @s slot.weapon.mainhand 0 {item} 64"));
        lines.extend(owned(["!place 2 -1 -2", "!place 2 -1 -1", "!place 2 -1 0", "!sneak", "", "!place 2 -1 1", "!unsneak", "", "fill 2 -26 -2 2 -24 1 air", "fill 2 -27 -2 2 -27 1 stone", ""]));
    }
    for (block, z) in BESIDE.iter().zip(-3..) {
        lines.push(format!("setblock 4 -26 {z} {block}"));
    }
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 oak_fence 64", "!place 3 -1 -3", "!place 3 -1 -2", "!place 3 -1 -1", "!place 3 -1 0", "!place 3 -1 1", "!place 3 -1 2", ""]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 cobblestone_wall 64", "!place 3 -1 3", "", "fill 3 -26 -3 4 -24 3 air", ""]));
    lines.extend(owned(["setblock 2 -24 -1 smooth_stone_slab [\"minecraft:vertical_half\"=\"top\"]", "setblock 2 -24 1 stone", "replaceitem entity @s slot.weapon.mainhand 0 glow_berries 64"]));
    for _ in 0..8 {
        lines.extend(owned(["!place 2 2 -1 down", "!place 2 2 1 down", "", "setblock 2 -25 -1 air", "setblock 2 -25 1 air"]));
    }
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 weeping_vines 64", "!place 2 2 1 down", "", "setblock 2 -25 1 air", "setblock 2 -27 3 stone"]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 twisting_vines 64", "!place 2 -1 3", "", "fill 2 -26 -1 2 -24 3 air", ""]));
    lines.extend(owned(["setblock 3 -26 -2 stone", "setblock 3 -26 2 stone", "replaceitem entity @s slot.weapon.mainhand 0 undyed_shulker_box 64", "!place 3 0 -2 up", "!place 3 0 -2 west", "!place 3 0 -2 south", ""]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 end_rod 64", "!place 3 0 2 up", "!place 3 0 2 west", "!place 3 0 2 north", "", "fill 2 -26 -3 4 -24 3 air", ""]));
    lines.extend(owned(["setblock 2 -26 -1 snow_layer [\"height\"=0]", "setblock 2 -26 1 snow_layer [\"height\"=1]", "replaceitem entity @s slot.weapon.mainhand 0 cobblestone 64", "!place 2 0 -1", "!place 2 0 1", "", "fill 2 -26 -1 2 -24 1 air", ""]));
    lines.extend(owned(["fill 3 -28 -2 5 -28 2 stone", "setblock 4 -28 1 sand", "fill 3 -27 -2 5 -27 2 water", ""]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 kelp 64", "!place 4 -2 -1", "", "setblock 4 -28 -1 magma", "", "", ""]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 tube_coral_fan 64", "!place 4 -2 1", ""]));
    waits(&mut lines, 3);
    lines.extend(owned(["fill 3 -27 -2 5 -16 2 air", "fill 3 -28 -2 5 -28 2 air", "fill 3 -27 -2 5 -27 2 stone", "clear @s", ""]));
    lines
}
