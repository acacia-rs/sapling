//! The fourteenth wave's group Y drills (research/parity/FOURTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27). `y3pin`: cows and a stand pinned by pistons against obsidian, a cow off the middle of its
//! row either way. `y3fling`: the actor (survival, no fall damage) on slime pushed east, then up. `y3auto`:
//! recipe-book crafts of sticks forged with Consumes from the inventory, the hotbar, an open chest and the
//! wrong item or count (`!auto`, crafting_auto.rs).

use super::creatures::{floor, waits, with};

const MOVES: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "MovePlayer", "SetEntityMotion", "UpdateBlock", "EntityEvent", "CorrectPlayerMovePrediction", "CommandOutput"];
const CRAFTS: &[&str] = &["ItemStackResponse", "InventorySlot", "InventoryContent", "ContainerOpen", "ContainerClose", "AddItemEntity", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "y3pin" => (pinned(), MOVES),
        "y3fling" => (flung(), MOVES),
        "y3auto" => (auto(), CRAFTS),
        _ => return None,
    })
}

fn piston(x: i32, y: i32, z: i32, facing: u8) -> String {
    format!("setblock {x} {y} {z} piston [\"facing_direction\"={facing}]")
}

const EAST: u8 = 4;
const UP: u8 = 1;

fn tidy(lines: Vec<String>) -> Vec<String> {
    with(lines, &["", "gamemode creative @s", "tp @s 0.5 -26 0.5 -90 0", "kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 stone", ""])
}

fn pinned() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "tp @s -6.5 -26 -6.5 -90 0"]);
    for (z, who, off) in [(-6, "cow", 0.5), (-3, "cow", 0.3), (0, "cow", 0.7), (3, "armor_stand", 0.3)] {
        lines.extend([piston(2, -26, z, EAST), format!("setblock 4 -26 {z} obsidian"), format!("summon {who} 3.5 -26 {}", z as f32 + off)]);
    }
    lines = with(lines, &["effect @e[type=cow] slowness 100 255 true", "", "setblock 2 -25 -6 redstone_block", "setblock 2 -25 -3 redstone_block", "setblock 2 -25 0 redstone_block", "setblock 2 -25 3 redstone_block"]);
    waits(&mut lines, 6);
    tidy(lines)
}

fn flung() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "gamerule falldamage false", "gamemode survival @s"]);
    lines.extend([piston(2, -26, -4, EAST), "setblock 3 -26 -4 slime".into(), "tp @s 3.5 -25 -3.5 -90 0".into(), String::new(), String::new(), "setblock 2 -25 -4 redstone_block".into()]);
    waits(&mut lines, 6);
    lines.extend([piston(6, -26, 2, UP), "setblock 6 -25 2 slime".into(), "tp @s 6.5 -24 2.5 -90 0".into(), String::new(), String::new(), "setblock 6 -27 2 redstone_block".into()]);
    waits(&mut lines, 8);
    tidy(with(lines, &["gamerule falldamage true"]))
}

/// Clears the actor and gives it `items` (`slot.hotbar 0 oak_planks 2`), then crafts `craft`.
fn case(lines: &mut Vec<String>, items: &[&str], craft: &str) {
    lines.extend(["clear @s".into(), "!forget".into()]);
    lines.extend(items.iter().map(|item| format!("replaceitem entity @s {item}")));
    // BDS crafts only on a screen with a craft handler (the inventory's or a table's); else status 7.
    lines.extend([String::new(), "!openinv".into(), String::new(), format!("!auto {craft}"), String::new(), "!close".into(), String::new()]);
}

fn auto() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -4.5 -26 0.5", "gamemode survival @s"]);
    let planks = "slot.hotbar 0 oak_planks";
    case(&mut lines, &[&format!("{planks} 2")], "stick 1 n0 n0");
    case(&mut lines, &[&format!("{planks} 1"), "slot.inventory 0 oak_planks 1"], "stick 1 n0 n9");
    case(&mut lines, &[&format!("{planks} 2")], "stick 1 h0 h0");
    case(&mut lines, &[&format!("{planks} 1"), "slot.inventory 0 oak_planks 1"], "stick 1 n0 i9");
    case(&mut lines, &["slot.hotbar 0 cobblestone 2"], "stick 1 n0 n0");
    case(&mut lines, &[&format!("{planks} 4")], "stick 1 n0*2");
    case(&mut lines, &[&format!("{planks} 4")], "stick 2 n0*2 n0*2");
    case(&mut lines, &[&format!("{planks} 2")], "stick 1 n0");
    case(&mut lines, &[&format!("{planks} 2")], "stick 1 n0 n0 n0");
    case(&mut lines, &[&format!("{planks} 2")], "stick 1 n0 n40");
    lines = with(lines, &["clear @s", "!forget", "setblock 2 -26 0 chest", "replaceitem block 2 -26 0 slot.container 0 oak_planks 2", "", "!click 2 0 0 up", "", ""]);
    lines = with(lines, &["!auto stick 1 c0 c0", "", "!close", ""]);
    tidy(with(lines, &["clear @s"]))
}
