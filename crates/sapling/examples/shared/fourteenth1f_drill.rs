//! The fourteenth wave's group F drills (research/parity/FOURTEENTH-BRIEF.md), on creature_drill.rs's floor (stone at
//! y -27). `ffpush`: a blocked piston unblocked with its power left on (obsidian ahead, then 13 stones), a chest
//! pushed while the actor has it open, and sand, a carpet and a stand on pushed stones. `ffshove`: stands and a cow on
//! slime, honey and stone pushed east and up, a cow and the survival actor pinned against obsidian. `ffcomp`:
//! a composter clicked from 6 with pumpkin pie and emptied, one fed by a hopper above and emptied by one below.
//! `ffcart`: hoppers filling and emptying carts and a hopper minecart under a chest and over a hopper; a comparator by
//! a detector rail; hopper minecarts empty, full and with one stack sped down a rail; a minecart round a curve.
//! `ffburn`: boats and carts in lava and in fire. `fftnt`: TNT minecarts in fire, lava, shot plainly and with a
//! flaming arrow, clicked with a fire charge, and hurt by `/damage` of each fire cause.

use super::creatures::{floor, waits, with};

const PUSH_SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "LevelEvent", "LevelSoundEvent", "AddEntity", "AddItemEntity", "RemoveEntity", "ContainerOpen", "ContainerClose", "InventoryContent",
    "MoveEntityDelta", "CommandOutput",
];
const SHOVE_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "EntityEvent", "UpdateAttributes", "SetHealth", "MovePlayer", "CorrectPlayerMovePrediction", "CommandOutput",
    "LevelSoundEvent", "BlockEntityData",
];
const COMP_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent", "AddItemEntity", "SetEntityMotion", "RemoveEntity", "CommandOutput", "InventorySlot"];
const CART_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "UpdateBlock", "UpdateSubchunkBlocks", "CommandOutput", "AddItemEntity", "BlockEntityData",
];
const BURN_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "UpdateBlock", "CommandOutput", "SetEntityMotion", "Animate",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ffpush" => (push(), PUSH_SHOWN),
        "ffshove" => (shove(), SHOVE_SHOWN),
        "ffcomp" => (compost(), COMP_SHOWN),
        "ffcart" => (carts(), CART_SHOWN),
        "ffburn" => (burn(), BURN_SHOWN),
        "fftnt" => (tnt(), BURN_SHOWN),
        _ => return None,
    })
}

fn piston(x: i32, y: i32, z: i32, facing: u8) -> String {
    format!("setblock {x} {y} {z} piston [\"facing_direction\"={facing}]")
}

const EAST: u8 = 4;
const UP: u8 = 1;

fn push() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -4.5 -26 0.5"]);
    lines.extend([piston(2, -26, -6, EAST), "setblock 3 -26 -6 stone".into(), "setblock 4 -26 -6 obsidian".into()]);
    lines.extend(["fill 3 -26 -4 15 -26 -4 stone".into(), piston(2, -26, -4, EAST), String::new()]);
    lines = with(lines, &["setblock 2 -25 -6 redstone_block", "setblock 2 -25 -4 redstone_block"]);
    waits(&mut lines, 2);
    lines.push("setblock 4 -26 -6 air".into());
    waits(&mut lines, 3);
    lines.push("setblock 15 -26 -4 air".into());
    waits(&mut lines, 3);
    lines.extend([piston(2, -26, 2, EAST), "setblock 3 -26 2 chest".into(), "replaceitem block 3 -26 2 slot.container 0 diamond 3".into(), String::new()]);
    lines = with(lines, &["!click 3 0 2", "", "setblock 2 -25 2 redstone_block"]);
    waits(&mut lines, 3);
    lines = with(lines, &["!close", "", "!click 4 0 2", "", "!close", ""]);
    for (z, on) in [(4, "setblock 7 -25 4 sand"), (6, "setblock 7 -25 6 white_carpet"), (8, "summon armor_stand 7.5 -25 8.5")] {
        lines.extend([piston(6, -26, z, EAST), format!("setblock 7 -26 {z} stone"), on.into()]);
    }
    lines = with(lines, &["", "fill 6 -25 4 6 -25 8 redstone_block"]);
    waits(&mut lines, 4);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 stone", ""])
}

const STILL: &str = "effect @e[type=cow] slowness 100 255 true";

fn shove() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "tp @s -6.5 -26 -4.5 -90 0"]);
    for (z, block, who) in [(-6, "slime", "armor_stand"), (-4, "honey_block", "armor_stand"), (-2, "stone", "armor_stand"), (0, "slime", "cow")] {
        lines.extend([piston(2, -26, z, EAST), format!("setblock 3 -26 {z} {block}"), format!("summon {who} 3.5 -25 {}", z as f32 + 0.5)]);
    }
    lines = with(lines, &[STILL, "", "fill 2 -25 -6 2 -25 0 redstone_block"]);
    waits(&mut lines, 4);
    for (x, block, who) in [(6, "slime", "armor_stand"), (8, "slime", "cow"), (10, "stone", "armor_stand"), (12, "honey_block", "armor_stand")] {
        lines.extend([piston(x, -26, 2, UP), format!("setblock {x} -25 2 {block}"), format!("summon {who} {x}.5 -24 2.5")]);
    }
    lines = with(lines, &[STILL, "", "fill 6 -27 2 12 -27 2 redstone_block"]);
    waits(&mut lines, 5);
    lines.extend([piston(2, -26, 4, EAST), "setblock 4 -26 4 obsidian".into(), "summon cow 3.5 -26 4.5".into(), STILL.into(), String::new(), "setblock 2 -25 4 redstone_block".into()]);
    waits(&mut lines, 6);
    lines.extend([piston(2, -26, 6, EAST), "setblock 4 -26 6 obsidian".into(), "gamemode survival @s".into(), "tp @s 3.5 -26 6.5 -90 0".into(), String::new(), "setblock 2 -25 6 redstone_block".into()]);
    waits(&mut lines, 6);
    with(lines, &["gamemode creative @s", "tp @s 0.5 -26 0.5 -90 0", "kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 stone", ""])
}

fn compost() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -4.5 -26 0.5", "replaceitem entity @s slot.weapon.mainhand 0 pumpkin_pie 16"]);
    let six = "setblock 2 -26 -3 composter [\"composter_fill_level\"=6]";
    lines = with(lines, &[six, "", "!click 2 0 -3"]);
    waits(&mut lines, 3);
    lines = with(lines, &["!click 2 0 -3", "", "", six, "", "!click 2 0 -3", "!click 2 0 -3"]);
    waits(&mut lines, 2);
    lines = with(lines, &["!click 2 0 -3", "", ""]);
    lines = with(lines, &[
        "kill @e[type=item]", "setblock 6 -27 0 hopper [\"facing_direction\"=0]", "setblock 6 -26 0 composter", "setblock 6 -25 0 hopper [\"facing_direction\"=0]",
        "replaceitem block 6 -25 0 slot.container 0 stone 2", "replaceitem block 6 -25 0 slot.container 1 pumpkin_pie 10",
    ]);
    waits(&mut lines, 16);
    lines = with(lines, &["setblock 10 -27 0 hopper [\"facing_direction\"=0]", "setblock 10 -26 0 composter [\"composter_fill_level\"=8]"]);
    waits(&mut lines, 3);
    lines = with(lines, &["setblock 6 -27 0 air destroy", "setblock 10 -27 0 air destroy", "setblock 6 -25 0 air destroy"]);
    waits(&mut lines, 2);
    with(lines, &["kill @e[type=!player]", "clear @s", "fill -8 -26 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 stone", ""])
}

fn rail(x: i32, z: i32, direction: u8) -> String {
    format!("setblock {x} -26 {z} rail [\"rail_direction\"={direction}]")
}

/// The `kind` at `x`, `z` on the floor: BDS names no vehicle by its summon, so `@e[name=..]` found none.
fn at(kind: &str, x: f32, z: f32) -> String {
    format!("@e[type={kind},x={x},y=-26,z={z},r=1.5]")
}

fn fill_cart(cart: &str, slots: std::ops::Range<u8>) -> Vec<String> {
    slots.map(|slot| format!("replaceitem entity {cart} slot.inventory {slot} dirt 64")).collect()
}

fn carts() -> Vec<String> {
    let mut lines = with(floor(), &["fill -8 -27 9 16 -27 12 stone", "fill -8 -26 9 16 -16 12 air", "tp CmdWatcher 0.5 -26 -1.5", "tp @s 0.5 -26 8.5 -90 0"]);
    lines.extend([rail(2, -7, 1), "summon chest_minecart c1 2.5 -26 -6.5".into()]);
    lines.extend(["setblock 6 -27 -7 hopper [\"facing_direction\"=0]".into(), rail(6, -7, 1), "summon chest_minecart c2 6.5 -26 -6.5".into(), format!("replaceitem entity {} slot.inventory 0 dirt 10", at("chest_minecart", 6.5, -6.5))]);
    lines.extend([rail(10, -7, 1), "summon hopper_minecart h1 10.5 -26 -6.5".into(), "setblock 10 -25 -7 chest".into(), "replaceitem block 10 -25 -7 slot.container 0 dirt 10".into()]);
    lines.extend(["setblock 14 -27 -7 hopper [\"facing_direction\"=0]".into(), rail(14, -7, 1), "summon hopper_minecart h2 14.5 -26 -6.5".into(), format!("replaceitem entity {} slot.inventory 0 dirt 10", at("hopper_minecart", 14.5, -6.5))]);
    lines = with(lines, &["setblock 2 -25 -7 hopper [\"facing_direction\"=0]", "replaceitem block 2 -25 -7 slot.container 0 dirt 10"]);
    waits(&mut lines, 8);
    lines = with(lines, &["kill @e[type=chest_minecart]", "kill @e[type=hopper_minecart]", "", "kill @e[type=item]"]);
    lines.extend([
        "setblock 2 -26 -4 detector_rail [\"rail_direction\"=1]".into(),
        "setblock 3 -26 -4 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\",\"output_subtract_bit\"=false]".into(),
        "setblock 4 -26 -4 redstone_wire".into(),
        "summon hopper_minecart h3 2.5 -26 -3.5".into(),
    ]);
    waits(&mut lines, 2);
    let h3 = at("hopper_minecart", 2.5, -3.5);
    lines.extend(fill_cart(&h3, 0..2));
    waits(&mut lines, 2);
    lines.extend(fill_cart(&h3, 2..3));
    waits(&mut lines, 2);
    lines = with(lines, &["kill @e[type=hopper_minecart]", "", "kill @e[type=item]", ""]);
    for z in [0, 2, 4, 6] {
        lines.extend([format!("setblock -8 -26 {z} stone"), format!("fill -7 -26 {z} -5 -26 {z} golden_rail [\"rail_direction\"=1]"), format!("fill -4 -26 {z} 13 -26 {z} rail [\"rail_direction\"=1]")]);
    }
    lines.extend(["fill 14 -26 0 14 -26 4 stone".into(), rail(14, 6, 7), "fill 14 -26 7 14 -26 11 rail [\"rail_direction\"=0]".into(), "setblock 14 -26 12 stone".into()]);
    lines.extend([rail(10, 10, 0), "summon minecart y1 10.5 -26 10.5".into()]);
    // On the golden rail by the stone, from which a powered one starts a cart standing on it.
    lines = with(lines, &["summon hopper_minecart d0 -6.5 -26 0.5", "summon hopper_minecart d1 -6.5 -26 2.5", "summon hopper_minecart d2 -6.5 -26 4.5", "summon minecart d3 -6.5 -26 6.5"]);
    lines.extend(fill_cart(&at("hopper_minecart", -6.5, 2.5), 0..5));
    lines.extend(fill_cart(&at("hopper_minecart", -6.5, 4.5), 0..1));
    lines = with(lines, &["", "fill -7 -27 0 -5 -27 6 redstone_block"]);
    waits(&mut lines, 8);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 12 air", "fill -8 -27 -8 16 -27 12 stone", "tp @s 0.5 -26 0.5 -90 0", ""])
}

fn burn() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "tp @s -6.5 -26 -6.5 -90 0", "fill 2 -28 -6 8 -28 -2 stone", "fill 2 -27 -6 8 -27 -2 lava", "fill 2 -27 2 8 -27 2 netherrack"]);
    lines = with(lines, &["setblock 2 -26 2 fire", "setblock 5 -26 2 fire", "setblock 8 -26 2 fire", ""]);
    lines = with(lines, &[
        "summon boat lb 3.5 -26 -4.5", "summon minecart lm 6.5 -26 -4.5", "summon chest_minecart lc 7.5 -26 -2.5", "summon boat fb 2.5 -26 2.5", "summon minecart fm 5.5 -26 2.5", "summon chest_boat fc 8.5 -26 2.5",
    ]);
    waits(&mut lines, 20);
    lines = with(lines, &["kill @e[type=!player]", "fill 2 -27 -6 8 -27 -2 stone", "fill 2 -26 2 8 -26 2 air", "fill 2 -27 2 8 -27 2 stone", ""]);
    for (i, cause) in ["fire", "lava", "fire_tick"].into_iter().enumerate() {
        let x = 2 + 4 * i as i32;
        lines.extend([format!("summon boat b{cause} {x}.5 -26 6.5"), format!("summon minecart m{cause} {x}.5 -26 4.5"), String::new()]);
        lines.extend([format!("damage {} 2 {cause}", at("boat", x as f32 + 0.5, 6.5)), format!("damage {} 2 {cause}", at("minecart", x as f32 + 0.5, 4.5)), String::new()]);
    }
    waits(&mut lines, 2);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "fill -8 -28 -8 16 -28 8 air", "fill -8 -27 -8 16 -27 8 stone", ""])
}

/// Each case on an obsidian floor, the cart 8 east of the actor; what is left is cleared after.
fn tnt() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "fill -8 -27 -8 16 -27 8 obsidian", "replaceitem entity @s slot.weapon.mainhand 0 bow", ""]);
    let clear = ["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 obsidian", "tp @s 0.5 -26 0.5 -90 0", ""];
    let cart = "summon tnt_minecart t 8.5 -26 0.5";
    let shot = ["", "!look 8 0.4 0", "!use", "", "!release"];
    let cases: [&[&str]; 6] = [
        &["setblock 8 -27 0 netherrack", "setblock 8 -26 0 fire", cart],
        &["fill 7 -28 -1 9 -28 1 obsidian", "setblock 8 -27 0 lava", cart],
        &[cart],
        &["enchant @s flame 1", cart],
        &["replaceitem entity @s slot.weapon.mainhand 0 fire_charge", "tp @s 6.5 -26 0.5 -90 0", cart, "", "!interact tnt_minecart"],
        &["summon tnt_minecart tf -1.5 -26 6.5", "summon tnt_minecart tl 2.5 -26 6.5", "summon tnt_minecart tt 6.5 -26 6.5", ""],
    ];
    for (i, case) in cases.into_iter().enumerate() {
        lines = with(lines, case);
        if i == 5 {
            let hurt = [("fire", -1.5), ("lava", 2.5), ("fire_tick", 6.5)].map(|(cause, x)| format!("damage {} 1 {cause}", at("tnt_minecart", x, 6.5)));
            lines.extend(hurt);
        }
        if (2..=3).contains(&i) {
            lines = with(lines, &shot);
        }
        waits(&mut lines, 6);
        lines = with(lines, &clear);
    }
    with(lines, &["fill -8 -27 -8 16 -27 8 stone", "clear @s", ""])
}
