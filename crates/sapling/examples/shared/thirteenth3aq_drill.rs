//! The thirteenth wave's group AQ drills (research/parity/THIRTEENTH-BRIEF.md): an allay handed dirt two
//! seconds after it appears, its item taken back, handed again and left by its owner 30 blocks off, then
//! given a dirt to carry as the drill ends, its world kept for `actor_dump` (`aq3allay`).

use crate::creatures::{floor, waits, with};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "aq3allay" => (allay(), ALLAY_SHOWN),
        "aq3exit" => (exits(), EXIT_SHOWN),
        _ => return None,
    })
}

const ALLAY_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "TakeItemEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "MobEquipment", "InventorySlot", "MoveEntityDelta",
    "SetEntityMotion", "MovePlayer", "CommandOutput",
];

const EXIT_SHOWN: &[&str] = &["AddEntity", "SetEntityLink", "MoveEntityDelta", "MovePlayer", "CorrectPlayerMovePrediction", "SetEntityData", "CommandOutput"];

/// Riders let off a still pig with lava north, stone north and lava south, and nothing about (run under
/// gdb on the exit search, tools/remote-drill.sh `BDS_GDB`); an allay summoned first, for its ambient check.
fn exits() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule dofiretick false", "summon allay 8.5 -26 4.5", "tp CmdWatcher 18.5 -26 -6.5", "fill -2 -27 -14 24 -27 -9 stone", "fill -2 -26 -14 24 -20 -9 air"]);
    let trials: [&[(i32, &str)]; 3] = [&[(-1, "lava")], &[(-1, "stone"), (1, "lava")], &[]];
    for (n, around) in trials.iter().enumerate() {
        let x = 1 + 2 * n as i32;
        lines.extend(["kill @e[type=pig]".to_owned(), format!("summon pig {x}.5 -26 -11.5 0 0 minecraft:on_saddled"), "effect @e[type=pig] slowness 30 255 true".to_owned(), String::new()]);
        lines.push("ride @s start_riding @e[type=pig,c=1] teleport_rider".to_owned());
        lines.extend(around.iter().map(|(dz, block)| format!("setblock {x} -26 {} {block}", -12 + dz)));
        lines.extend(["".to_owned(), "ride @s stop_riding".to_owned(), String::new(), String::new(), "fill -2 -26 -14 24 -20 -9 air".to_owned()]);
    }
    with(lines, &["kill @e[type=!player]", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

/// The allay put back beside the actor, then clicked.
fn click(lines: &mut Vec<String>, before: &str) {
    lines.extend([before.to_owned(), "tp @e[type=allay] -0.5 -25.5 1.5".to_owned(), String::new(), "!interact allay".to_owned()]);
}

fn allay() -> Vec<String> {
    let mut lines = with(floor(), &["fill 17 -27 -8 40 -27 8 stone", "fill 17 -26 -8 40 -16 8 air", "tp CmdWatcher -6.5 -26 -6.5", "gamemode survival @s", "replaceitem entity @s slot.weapon.mainhand 0 dirt 4"]);
    lines.extend(["summon allay 1.5 -26 0.5".to_owned(), "effect @e[type=allay] slowness 120 255 true".to_owned()]);
    click(&mut lines, "");
    waits(&mut lines, 8);
    click(&mut lines, "replaceitem entity @s slot.weapon.mainhand 0 air");
    waits(&mut lines, 10);
    click(&mut lines, "replaceitem entity @s slot.weapon.mainhand 0 dirt 4");
    lines.extend(["effect @e[type=allay] clear".to_owned(), String::new(), "tp @s 30.5 -26 0.5".to_owned()]);
    waits(&mut lines, 12);
    lines.extend(["tp @s 0.5 -26 0.5".to_owned(), "effect @e[type=allay] slowness 120 255 true".to_owned(), "tp @e[type=allay] 6.5 -25.5 0.5".to_owned(), String::new()]);
    lines.extend(["setblock 6 -26 0 dirt".to_owned(), "setblock 6 -26 0 air destroy".to_owned(), String::new(), "tp @s 30.5 -26 0.5".to_owned()]);
    waits(&mut lines, 2);
    lines
}
