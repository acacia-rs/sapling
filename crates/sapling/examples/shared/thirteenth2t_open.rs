//! Group T's `z2open`: the sub-items group P left open, from the actor at 0.5 -26 0.5 (clicks are offsets from
//! its cell). A wall frame a comparator reads through its block, clicked in and turned (z -3); a jukebox given a
//! disc and clicked again (z -1); a chiseled bookshelf given two books, a comparator behind (z 1); a copper golem
//! statue clicked twice (z 3); a piston whose power goes the next command (z -6); a dropper into a full chest,
//! pulsed once and then twice at once (z 6); a crafter making planks (z 4, x 6); an anvil on the actor in
//! survival; a bed's foot clicked in the Nether.

use super::owned;
use super::super::creatures::{floor, waits};

const OPEN_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "BlockEvent", "LevelEvent", "LevelSoundEvent", "AddEntity", "AddItemEntity", "UpdateAttributes", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    (name == "z2open").then(|| (open(), OPEN_SHOWN))
}

fn open() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["setblock 3 -26 -3 stone", "setblock 2 -26 -3 frame [\"facing_direction\"=4]", "setblock 4 -26 -3 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\"]", "setblock 5 -26 -3 redstone_wire"]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 apple 1", "", "!click 2 0 -3 west", "", "!click 2 0 -3 west", "", "!click 2 0 -3 west", ""]));
    lines.extend(owned(["setblock 2 -26 -1 jukebox", "replaceitem entity @s slot.weapon.mainhand 0 music_disc_13 1", "", "!click 2 0 -1 west", "", "!click 2 0 -1 west", ""]));
    lines.extend(owned(["setblock 2 -26 1 chiseled_bookshelf [\"direction\"=1]", "setblock 3 -26 1 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\"]", "setblock 4 -26 1 redstone_wire"]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 book 2", "", "!click 2 0 1 west", "", "!click 2 0 1 west", ""]));
    lines.extend(owned(["setblock 2 -26 3 copper_golem_statue", "clear @s", "", "!click 2 0 3 west", "", "!click 2 0 3 west", ""]));
    lines.extend(owned(["kill @e[type=item]", "fill 2 -26 -4 6 -24 4 air", ""]));
    lines.extend(owned(["setblock 3 -26 -6 piston [\"facing_direction\"=1]", "setblock 3 -25 -6 stone", "", "setblock 2 -26 -6 redstone_block", "setblock 2 -26 -6 air", ""]));
    waits(&mut lines, 2);
    lines.extend(owned(["fill 2 -26 -6 3 -23 -6 air", "setblock 3 -26 6 chest", "setblock 2 -26 6 dropper [\"facing_direction\"=5]", "replaceitem block 2 -26 6 slot.container 0 dirt 5"]));
    lines.extend((0..27).map(|slot| format!("replaceitem block 3 -26 6 slot.container {slot} stone 64")));
    lines.extend(owned(["", "setblock 2 -26 7 redstone_block", "", "setblock 2 -26 7 air", "", "setblock 2 -26 7 redstone_block", "setblock 2 -26 7 air", "setblock 2 -26 7 redstone_block", "setblock 2 -26 7 air", ""]));
    waits(&mut lines, 1);
    lines.extend(owned(["kill @e[type=item]", "fill 2 -26 5 3 -26 7 air", "setblock 5 -26 4 crafter [\"orientation\"=\"east_up\"]", "replaceitem block 5 -26 4 slot.container 0 oak_log 1", "", "setblock 5 -26 5 redstone_block", ""]));
    waits(&mut lines, 1);
    lines.extend(owned(["kill @e[type=item]", "fill 5 -26 4 6 -26 5 air", ""]));
    lines.extend(owned(["gamemode survival @s", "setblock 0 -16 0 anvil", ""]));
    waits(&mut lines, 2);
    lines.extend(owned(["gamemode creative @s", "fill 0 -26 0 0 -16 0 air", "effect @s instant_health 1 5", ""]));
    lines.extend(owned(["execute in nether run tp @s 0.5 120 0.5 -90 0", "execute in nether run fill -3 118 -3 6 124 3 air", "execute in nether run fill -3 118 -3 6 118 3 obsidian", ""]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 bed 1", "!place 2 -1 0", "", "!click 2 0 0", ""]));
    waits(&mut lines, 1);
    lines.extend(owned(["execute in overworld run tp @s 0.5 -26 0.5 -90 0", "kill @e[type=item]", "clear @s", ""]));
    lines
}
