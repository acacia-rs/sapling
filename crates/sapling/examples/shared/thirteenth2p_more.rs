//! Group P's second-round drill (`p2more`): what p2place and p2liquid could not reach, each scene near the
//! actor and cleared after it.

use super::owned;
use super::super::creatures::{floor, waits};

const MORE_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "BlockEvent", "LevelEvent", "LevelSoundEvent", "AddEntity", "RemoveEntity", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    match name {
        "p2more" => Some((more(), MORE_SHOWN)),
        "p2last" => Some((last(), MORE_SHOWN)),
        _ => None,
    }
}

/// Sugar cane put by hand on seven soils beside water (x 2, z -3 to 3); an empty bucket on a full water
/// cauldron (x 2, z 5); a book and quill put on a lectern a comparator reads (x 2, z -5); coral left dry in a
/// glass ring (x 6); a charged respawn anchor clicked with `respawnblocksexplode` off, then on (x 2, z 7).
fn last() -> Vec<String> {
    const SOILS: [&str; 7] = ["podzol", "mycelium", "moss_block", "mud", "suspicious_sand", "sand", "dirt"];
    let mut lines = floor();
    lines.extend(owned(["fill 2 -28 -3 3 -28 3 stone", "fill 3 -27 -3 3 -27 3 water"]));
    for (soil, z) in SOILS.iter().zip(-3..) {
        lines.push(format!("setblock 2 -27 {z} {soil}"));
    }
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 sugar_cane 64", ""]));
    lines.extend((-3..=3).map(|z| format!("!place 2 -1 {z}")));
    lines.extend(owned(["", "fill 2 -26 -3 2 -24 3 air", "fill 2 -27 -3 3 -27 3 stone", ""]));
    lines.extend(owned(["setblock 2 -26 5 cauldron [\"cauldron_liquid\"=\"water\",\"fill_level\"=6]", "replaceitem entity @s slot.weapon.mainhand 0 bucket 1", "!click 2 0 5 west", "", "setblock 2 -26 5 air", ""]));
    lines.extend(owned(["setblock 2 -26 -5 lectern", "setblock 3 -26 -5 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\"]", "setblock 4 -26 -5 redstone_wire"]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 writable_book 1", "!click 2 0 -5 west", "", "", "fill 2 -26 -5 4 -26 -5 air", ""]));
    lines.extend(owned(["fill 5 -26 -1 7 -26 3 glass", "setblock 6 -26 0 tube_coral_block", "setblock 6 -26 2 tube_coral", "setblock 6 -26 1 air"]));
    waits(&mut lines, 12);
    lines.extend(owned(["fill 5 -26 -1 7 -26 3 stone", "fill 5 -26 -1 7 -26 3 air", "tp @s 0.5 -26 0.5 -90 0", "gamerule respawnblocksexplode false"]));
    lines.extend(owned(["setblock 2 -26 2 respawn_anchor [\"respawn_anchor_charge\"=1]", "clear @s", "!click 2 0 2 west", "", "gamerule respawnblocksexplode true", "!click 2 0 2 west", "", ""]));
    lines.extend(owned(["kill @e[type=!player]", "fill -2 -26 -8 10 -20 8 air", "fill -2 -27 -8 10 -27 8 stone", "clear @s", ""]));
    lines
}

/// Stone put on the west faces of snow one and two layers deep (z -1, 1); a shulker box put up, west and south
/// (x 3, z -2); a note block under each head in turn, clicked (x 3, z 2); then by `tp`s: an empty dispenser put
/// beside a block of redstone (x 5); kelp in a glass well whose floor turns to magma (x 7); coral left dry (x 10);
/// still water beside an upside-down stair and a top slab, and lava and water two apart, woken by glass over
/// them (x 4 to 6, z -3 to 2); a charged respawn anchor clicked with `respawnblocksexplode` off, then on (x 3, z 5).
/// Places and clicks are offsets from the actor's cell.
fn more() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["setblock 2 -26 -1 snow_layer [\"height\"=0]", "setblock 2 -26 1 snow_layer [\"height\"=1]", "replaceitem entity @s slot.weapon.mainhand 0 cobblestone 64"]));
    lines.extend(owned(["!place 2 0 -1 west", "!place 2 0 1 west", "", "fill 1 -26 -1 2 -24 1 air", ""]));
    lines.extend(owned(["setblock 3 -26 -2 stone", "replaceitem entity @s slot.weapon.mainhand 0 undyed_shulker_box 64", "!place 3 0 -2 up", "!place 3 0 -2 west", "!place 3 0 -2 south", ""]));
    lines.extend(owned(["fill 2 -26 -3 3 -24 -1 air", "setblock 3 -26 2 noteblock", "clear @s", "!click 3 0 2 west", ""]));
    for head in ["zombie_head", "skeleton_skull", "creeper_head", "piglin_head", "dragon_head", "wither_skeleton_skull", "player_head"] {
        lines.extend([format!("setblock 3 -25 2 {head}"), "!click 3 0 2 west".to_owned(), String::new()]);
    }
    lines.extend(owned(["fill 3 -26 2 3 -25 2 air", "tp @s 3.5 -26 -1.5 -90 0", "setblock 6 -26 -3 redstone_block", "replaceitem entity @s slot.weapon.mainhand 0 dispenser 4", "!place 2 -1 -1", ""]));
    waits(&mut lines, 2);
    lines.extend(owned(["fill 5 -26 -3 6 -26 -3 air", "fill 6 -26 2 8 -24 4 glass", "fill 7 -26 3 7 -24 3 water", "setblock 7 -26 3 kelp", ""]));
    waits(&mut lines, 2);
    lines.extend(owned(["setblock 7 -27 3 magma", ""]));
    waits(&mut lines, 2);
    // A cleared bubble column left its water behind and flooded the floor; stone over it first.
    lines.extend(owned(["setblock 7 -27 3 stone", "fill 6 -26 2 8 -24 4 stone", "fill 6 -26 2 8 -24 4 air", "", "tp @s 8.5 -26 0.5 -90 0"]));
    lines.extend(owned(["setblock 10 -26 -2 tube_coral_fan", "setblock 10 -26 0 tube_coral_block", "setblock 10 -26 2 tube_coral", ""]));
    waits(&mut lines, 8);
    lines.extend(owned(["fill 10 -26 -2 10 -26 2 air", "tp @s 0.5 -26 0.5 -90 0", "setblock 4 -26 -3 water", "setblock 5 -26 -3 oak_stairs [\"upside_down_bit\"=true,\"weirdo_direction\"=0]"]));
    lines.extend(owned(["setblock 4 -26 -1 water", "setblock 5 -26 -1 smooth_stone_slab [\"minecraft:vertical_half\"=\"top\"]", "setblock 4 -26 2 lava", "setblock 6 -26 2 water", ""]));
    lines.extend(owned(["setblock 4 -25 -3 glass", "setblock 4 -25 -1 glass", "setblock 4 -25 2 glass", "setblock 6 -25 2 glass", ""]));
    lines.extend(owned(["fill 1 -26 -7 11 -24 7 stone", "fill 1 -26 -7 11 -24 7 air", "fill -1 -26 -7 0 -24 -1 air", ""]));
    lines.extend(owned(["gamerule respawnblocksexplode false", "setblock 3 -26 5 respawn_anchor [\"respawn_anchor_charge\"=1]", "!click 3 0 5 west", ""]));
    lines.extend(owned(["gamerule respawnblocksexplode true", "!click 3 0 5 west", "", "", "kill @e[type=!player]", "fill -2 -26 0 8 -20 10 air", "fill -2 -28 0 8 -27 10 stone", "clear @s", ""]));
    lines
}
