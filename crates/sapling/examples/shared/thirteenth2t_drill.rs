//! The thirteenth wave's group T drills (research/parity/THIRTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27, the actor at 0.5 -26 0.5 facing east). `z2bolt`: where a bolt's vibration starts, and
//! waxed rods struck. `z2wire`: string put by hand between hooks, alone, beside string and by one hook.
//! `z2sculk`: wool between a sensor and a block put, wool and carpet put, and a sensor's power into the
//! block it stands on. `z2hand` (thirteenth2t_hand.rs): blocks with entities put by hand.

#[path = "thirteenth2t_hand.rs"]
mod hand;

use super::creatures::{floor, waits};

const BOLT_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "LevelEvent", "LevelEventGeneric", "LevelSoundEvent", "AddEntity", "CommandOutput"];
const WIRE_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "LevelSoundEvent", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "z2bolt" => (bolt(), BOLT_SHOWN),
        "z2wire" => (wire(), WIRE_SHOWN),
        "z2sculk" => (sculk(), BOLT_SHOWN),
        _ => return hand::drill(name),
    })
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// A sensor at 12 -26 -6 (its particle names each vibration's origin) hears a bolt over the floor at 12 -3
/// and one in a rod's cell at 8 -6; then bolts in the cells of a waxed rod (x 2), a waxed rod on oxidized
/// copper (x 4) and a plain exposed rod (x 6), and one over the waxed rod.
fn bolt() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["gamerule dofiretick false", "setblock 12 -26 -6 sculk_sensor", ""]));
    waits(&mut lines, 2);
    lines.extend(owned(["summon lightning_bolt 12.5 -26 -2.5", ""]));
    waits(&mut lines, 4);
    lines.extend(owned(["setblock 8 -26 -6 lightning_rod", ""]));
    waits(&mut lines, 3);
    lines.extend(owned(["summon lightning_bolt 8.5 -26 -5.5", ""]));
    waits(&mut lines, 4);
    lines.extend(owned(["setblock 12 -26 -6 air", "setblock 8 -26 -6 air", "setblock 2 -26 -6 waxed_lightning_rod", "setblock 4 -27 -6 oxidized_copper", "setblock 4 -26 -6 waxed_exposed_lightning_rod", "setblock 6 -26 -6 exposed_lightning_rod", ""]));
    for x in [2, 4, 6] {
        lines.extend([format!("summon lightning_bolt {x}.5 -26 -5.5"), String::new()]);
        waits(&mut lines, 1);
    }
    lines.extend(owned(["summon lightning_bolt 2.5 -25 -5.5", ""]));
    waits(&mut lines, 1);
    lines.extend(owned(["kill @e[type=!player]", "fill 0 -26 -8 14 -24 -1 air", "fill 0 -27 -8 14 -27 -1 stone", "gamerule dofiretick true", ""]));
    lines
}

/// From 4.5 -26 -1.5 facing north: hooks put by hand on stone at x 2 and 6 (z -4) and string between them;
/// string alone (4 0), a second beside it (5 0), string by a lone hook (z 2); then string set by command
/// alone (7 2) and beside string (6 0).
fn wire() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["setblock 2 -26 -4 stone", "setblock 6 -26 -4 stone", "setblock 2 -26 2 stone", "replaceitem entity @s slot.weapon.mainhand 0 tripwire_hook 4", ""]));
    lines.extend(owned(["tp @s 4.5 -26 -1.5 180 0", "", "!place 2 0 -2 west", "!place -2 0 -2 east", "!place -2 0 4 east", ""]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 string 8", "!place 0 -1 -2", "", "!place 0 -1 2", "", "!place 1 -1 2", "", "!place 0 -1 4", ""]));
    lines.extend(owned(["setblock 7 -26 2 trip_wire", "setblock 6 -26 0 trip_wire", ""]));
    waits(&mut lines, 1);
    lines.extend(owned(["tp @s 0.5 -26 0.5 -90 0", "fill 1 -26 -5 8 -24 3 air", "clear @s", ""]));
    lines
}

/// Blocks put by hand from 1.5 -26 -3.5 (a set block is heard by no sensor, as the first run showed): a
/// sensor at 6 -26 -4 hears stone put at 2 -26 -4 through a wool wall at x 4, then stone without it, then
/// wool and carpet put; then a sensor on a stone block at 6 -25 4 with a lamp beside that block (x 7), set
/// off by stone put at 2 -26 3.
fn sculk() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["fill 4 -26 -6 4 -24 -2 white_wool", "setblock 6 -26 -4 sculk_sensor", "tp @s 1.5 -26 -3.5 -90 0", "replaceitem entity @s slot.weapon.mainhand 0 stone 8", ""]));
    waits(&mut lines, 3);
    for (step, item) in [("!place 1 -1 0", ""), ("fill 4 -26 -6 4 -24 -2 air", ""), ("!place 1 -1 1", ""), ("!place 1 -1 2", "white_wool"), ("!place 1 -1 -1", "white_carpet")] {
        if !item.is_empty() {
            lines.push(format!("replaceitem entity @s slot.weapon.mainhand 0 {item} 1"));
        }
        lines.extend([step.to_string(), String::new()]);
        waits(&mut lines, 3);
    }
    lines.extend(owned(["fill 2 -26 -6 6 -24 -2 air", "setblock 6 -26 4 stone", "setblock 7 -26 4 redstone_lamp", "setblock 6 -25 4 sculk_sensor", "tp @s 1.5 -26 3.5 -90 0", "replaceitem entity @s slot.weapon.mainhand 0 stone 8", ""]));
    waits(&mut lines, 3);
    lines.extend(owned(["!place 1 -1 0", ""]));
    waits(&mut lines, 3);
    lines.extend(owned(["fill 2 -26 2 7 -24 5 air", "tp @s 0.5 -26 0.5 -90 0", "clear @s", ""]));
    lines
}
