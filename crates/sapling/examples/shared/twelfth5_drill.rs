//! The twelfth wave's group 5 drills (research/parity/TWELFTH-BRIEF.md): what hanging signs, rising rails
//! and small dripleaves need to stay, and the sounds of coral written and dying (`t5holds`).

use super::creatures::{waits, with};

const SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent", "BlockEntityData", "AddItemEntity", "CommandOutput"];
const SIGN: &str = "oak_hanging_sign [\"facing_direction\"=2,\"ground_sign_direction\"=8,\"hanging\"=true,\"attached_bit\"=true]";

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "t5holds" => (holds(), SHOWN),
        _ => return None,
    })
}

fn set(x: i32, y: i32, z: i32, block: &str) -> String {
    format!("setblock {x} {y} {z} {block}")
}

/// The bots in chunk 0,0 of the flat world on a stone floor at y -61, nothing over it.
fn bare() -> Vec<String> {
    let lines = ["kill @e[type=!player]", "gamerule domobspawning false", "weather clear", "gamemode creative @s", "gamemode creative CmdWatcher", "clear @s"];
    with(lines.map(String::from).into(), &["tp @s 10.5 -60 22.5", "tp CmdWatcher 11.5 -60 22.5", "fill 0 -61 0 20 -61 20 stone", "fill 0 -60 0 20 -55 20 air", ""])
}

/// Row z 2: hanging signs under stone (taken away after), a chain, a fence and a top slab, one in air
/// and a wall one in air, a row of stone set and taken beside them. Row z 6: rising rails with and
/// without the cell they rise to. Row z 10: small dripleaves on stone, clay, moss, dry dirt and dirt
/// under water; an upper half alone; the clay one's upper taken. Row z 16: coral plant, fan, dead
/// plant, block and a plant in water set, then left 12 s to die.
fn holds() -> Vec<String> {
    let mut lines = bare();
    let plain = SIGN.replace("\"attached_bit\"=true", "\"attached_bit\"=false");
    for (x, above) in [(2, "stone"), (6, "iron_chain"), (8, "oak_fence"), (10, "smooth_stone_slab [\"minecraft:vertical_half\"=\"top\"]")] {
        lines.extend([set(x, -59, 2, above), set(x, -60, 2, SIGN)]);
    }
    lines.extend([set(4, -59, 2, "stone"), set(4, -60, 2, &plain), set(12, -60, 2, SIGN), set(14, -60, 2, "oak_hanging_sign [\"facing_direction\"=2,\"hanging\"=false]"), String::new()]);
    lines.extend([set(2, -59, 2, "air"), set(4, -59, 2, "air"), String::new(), "fill 5 -60 3 15 -60 3 stone".into(), "fill 5 -60 3 15 -60 3 air".into(), String::new()]);
    lines.extend([set(3, -60, 6, "stone"), set(2, -60, 6, "activator_rail [\"rail_direction\"=2]"), set(7, -60, 6, "stone"), set(6, -60, 6, "rail [\"rail_direction\"=2]")]);
    lines.extend([set(10, -60, 6, "activator_rail [\"rail_direction\"=4]"), set(13, -60, 6, "stone"), set(14, -60, 6, "golden_rail [\"rail_direction\"=3]"), String::new()]);
    lines.extend([set(3, -60, 6, "air"), set(7, -60, 6, "air"), String::new()]);
    let leaf = "small_dripleaf_block [\"minecraft:cardinal_direction\"=\"north\",\"upper_block_bit\"=false]";
    lines.extend([set(6, -61, 10, "clay"), set(10, -61, 10, "moss_block"), set(14, -61, 10, "dirt"), set(18, -61, 10, "dirt"), set(18, -60, 10, "water"), String::new()]);
    for x in [2, 6, 10, 14, 18] {
        lines.extend([set(x, -60, 10, leaf), String::new()]);
    }
    lines.extend([set(2, -60, 12, &leaf.replace("=false", "=true")), String::new(), set(6, -59, 10, "air"), String::new(), String::new()]);
    lines.extend([set(12, -60, 16, "water"), String::new()]);
    for (x, coral) in [(2, "tube_coral"), (4, "tube_coral_fan"), (6, "dead_tube_coral"), (8, "tube_coral_block"), (12, "tube_coral")] {
        lines.push(set(x, -60, 16, coral));
    }
    waits(&mut lines, 12);
    with(lines, &["fill 0 -60 0 20 -55 20 air", "fill 0 -61 0 20 -61 20 stone", "kill @e[type=item]", ""])
}
