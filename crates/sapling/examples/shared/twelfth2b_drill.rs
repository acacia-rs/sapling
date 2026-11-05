//! The twelfth wave's group 3 drills (research/parity/TWELFTH-BRIEF.md): the random-tick pace by a
//! chunk's height (`g3rate`); coral fans placed by hand and a sponge's soaking (`g3fan`).

use super::creatures::{waits, with};

const SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent", "BlockEvent", "BlockEntityData", "AddEntity", "AddItemEntity", "CommandOutput"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "g3rate" => (rate(), SHOWN),
        "g3fan" => (fans(), SHOWN),
        _ => return None,
    })
}

/// Tube coral fans put by hand on a stone floor, the actor turned south, west, north and east, then one
/// into a water source; a sponge set dry, water set beside it, then a sponge set into water.
fn fans() -> Vec<String> {
    let mut lines = with(bare(), &["fill 0 -61 0 20 -61 20 stone", "fill 0 -60 0 20 -55 20 air", "replaceitem entity @s slot.weapon.mainhand 0 tube_coral_fan 8", ""]);
    for (yaw, offset) in [(0, "0 -1 2"), (90, "-2 -1 0"), (180, "0 -1 -2"), (270, "2 -1 0")] {
        lines.extend([format!("tp @s 10.5 -60 10.5 {yaw} 60"), String::new(), format!("!place {offset} up"), String::new()]);
    }
    lines.extend(["tp @s 10.5 -60 10.5 0 60", "fill 9 -61 13 11 -61 15 stone", "setblock 10 -60 13 water", "", "!place 0 -1 3 up", "", ""].map(String::from));
    lines.extend(["setblock 4 -60 4 sponge", "", "setblock 5 -60 4 water", "", "", "setblock 4 -60 8 water", "", "setblock 4 -60 8 sponge", "", ""].map(String::from));
    with(lines, &["fill 0 -60 0 20 -55 20 air", ""])
}

/// The bots in chunk 0,0 of the flat world; nothing set above its ground.
fn bare() -> Vec<String> {
    ["kill @e[type=!player]", "gamerule domobspawning false", "weather clear", "gamemode creative @s", "gamemode creative CmdWatcher", "clear @s", "tp @s 8.5 -60 8.5", "tp CmdWatcher 8.5 -60 8.5", ""]
        .map(String::from)
        .into()
}

/// A layer of 256 lit redstone ores at y -60 in each of four chunks whose highest block is in
/// section 0 (1,0), 1 (0,1), 2 (-1,0) and 22 (1,1), by a stone set high in each; three rounds of
/// `randomtickspeed` 50 for 12 s. Each ore's UpdateBlock times its life.
fn rate() -> Vec<String> {
    let mut lines = with(bare(), &["gamerule randomtickspeed 0", "setblock 8 -40 24 stone", "setblock -8 -20 8 stone", "setblock 24 300 24 stone", ""]);
    let chunks = [[16, 0], [0, 16], [-16, 0], [16, 16]];
    for _ in 0..3 {
        lines.extend(chunks.map(|[x, z]| format!("fill {x} -60 {z} {} -60 {} lit_redstone_ore", x + 15, z + 15)));
        lines.extend(["gamerule randomtickspeed 50".to_owned(), String::new()]);
        waits(&mut lines, 12);
        lines.push("gamerule randomtickspeed 0".into());
        lines.extend(chunks.map(|[x, z]| format!("fill {x} -60 {z} {} -60 {} air", x + 15, z + 15)));
        lines.push(String::new());
    }
    with(lines, &["gamerule randomtickspeed 1"])
}
