//! The thirteenth wave's group AP drills (research/parity/THIRTEENTH-BRIEF.md): an allay handed dirt
//! collects dirt dropped about it and brings it to its owner (`ap3give`), then to a note block played
//! near it (`ap3note`).

use crate::creatures::{floor, waits, with};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ap3give" => (give(false), ALLAY_SHOWN),
        "ap3note" => (give(true), ALLAY_SHOWN),
        _ => return None,
    })
}

const ALLAY_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "TakeItemEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "BlockEvent", "UpdateBlock", "MobEquipment",
    "InventorySlot", "MoveEntityDelta", "SetEntityMotion", "MovePlayer", "CommandOutput",
];

/// One dirt dropped at `x z` on the floor, as a broken block drops it.
fn dirt_at(lines: &mut Vec<String>, x: i32, z: i32) {
    lines.push(format!("setblock {x} -26 {z} dirt"));
    lines.push(format!("setblock {x} -26 {z} air destroy"));
}

/// An allay handed one of 4 dirt by the survival actor; a dirt dropped 9 off, then two more and a
/// cobblestone 12 off the other way, each watched 12 s. With `note`, a note block 6 off the allay's
/// start is played by a redstone block put under it before a last dirt is dropped.
fn give(note: bool) -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "gamemode survival @s", "replaceitem entity @s slot.weapon.mainhand 0 dirt 4", "summon allay 1.5 -26 0.5", "effect @e[type=allay] slowness 120 255 true", ""]);
    // Held until handed: a hovering allay drifts past the probe's 3-block reach (sapling's ignores
    // slowness and drifted 1.3 blocks to +x -z in the second's wait, so it is put the other side).
    lines.extend(["tp @e[type=allay] -0.5 -25.5 1.5".to_owned(), String::new(), "!interact allay".to_owned(), "effect @e[type=allay] clear".to_owned(), String::new()]);
    waits(&mut lines, 3);
    if note {
        lines.extend(["setblock 4 -26 -6 noteblock".to_owned(), "setblock 4 -27 -6 redstone_block".to_owned(), String::new()]);
        waits(&mut lines, 1);
    }
    dirt_at(&mut lines, 9, 4);
    waits(&mut lines, 12);
    dirt_at(&mut lines, -6, -5);
    dirt_at(&mut lines, -7, -5);
    lines.extend(["setblock -6 -26 -3 stone".to_owned(), "setblock -6 -26 -3 air destroy".to_owned()]);
    waits(&mut lines, 12);
    if note {
        lines.extend(["setblock 4 -27 -6 stone".to_owned(), "setblock 4 -27 -6 redstone_block".to_owned()]);
        dirt_at(&mut lines, 12, 6);
        waits(&mut lines, 12);
    }
    with(lines, &["kill @e[type=!player]", "fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -16 8 air", ""])
}
