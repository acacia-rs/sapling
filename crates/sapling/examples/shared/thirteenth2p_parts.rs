//! Group P's redstone, TNT, hopper and growth drills (`p2rs`, `p2tnt`, `p2hopper`, `p2grow`).

use super::owned;
use super::super::creatures::{floor, waits};

const RS_SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "CommandOutput", "BlockEvent", "BlockEntityData"];
const TNT_SHOWN: &[&str] = &["CommandOutput", "AddEntity", "RemoveEntity", "SetEntityData", "UpdateSubchunkBlocks", "UpdateBlock"];
const HOPPER_SHOWN: &[&str] = &["CommandOutput", "ContainerOpen", "InventorySlot", "InventoryContent", "AddItemEntity", "RemoveEntity", "UpdateBlock"];
const GROW_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "p2rs" => (redstone(), RS_SHOWN),
        "p2tnt" => (tnt_fuses(), TNT_SHOWN),
        "p2hopper" => (hoppers(), HOPPER_SHOWN),
        "p2grow" => (growing(), GROW_SHOWN),
        _ => return None,
    })
}

/// Redstone parts along z rows on the floor: an observer's pulse into a delay-4 repeater and lamp (z -6);
/// two observers watching each other, a lamp behind one (z -3); a piston set beside a block of redstone,
/// and one an observer's pulse powers (z 0, 1); a sticky piston drawing back from slime held by obsidian
/// (x 9); a piston pushing slime with a torch on it (x 12); a comparator with a lit repeater at its side,
/// in compare then subtract mode (z 4); one reading a cauldron of lava, then of powder snow (x 6).
fn redstone() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned([
        "setblock 1 -26 -6 redstone_lamp",
        "setblock 2 -26 -6 unpowered_repeater [\"minecraft:cardinal_direction\"=\"east\",\"repeater_delay\"=3]",
        "setblock 3 -26 -6 observer [\"minecraft:facing_direction\"=\"east\"]",
        "",
        "setblock 4 -26 -6 stone",
        "",
        "",
        "setblock 2 -26 -3 redstone_lamp",
        "setblock 3 -26 -3 observer [\"minecraft:facing_direction\"=\"east\"]",
        "",
        "setblock 4 -26 -3 observer [\"minecraft:facing_direction\"=\"west\"]",
        "",
        "",
        "setblock 4 -26 -3 air",
        "",
        "setblock 3 -26 0 redstone_block",
        "setblock 3 -26 2 stone",
        "setblock 3 -26 1 piston [\"facing_direction\"=2]",
        "",
        "setblock 6 -26 2 stone",
        "setblock 6 -26 1 piston [\"facing_direction\"=2]",
        "setblock 6 -26 0 observer [\"minecraft:facing_direction\"=\"north\"]",
        "",
        "setblock 6 -26 -1 stone",
        "",
        "",
        "setblock 9 -26 0 sticky_piston [\"facing_direction\"=2]",
        "setblock 9 -26 -1 redstone_block",
        "",
        "setblock 9 -26 2 slime",
        "setblock 10 -26 2 obsidian",
        "",
        "setblock 9 -26 -1 air",
        "",
        "",
        "setblock 12 -26 -3 piston [\"facing_direction\"=2]",
        "setblock 12 -26 -2 slime",
        "setblock 12 -25 -2 torch",
        "",
        "setblock 12 -26 -4 redstone_block",
        "",
        "",
        "setblock 2 -26 4 redstone_block",
        "setblock 4 -26 4 redstone_lamp",
        "setblock 3 -26 6 redstone_block",
        "setblock 3 -26 4 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\",\"output_subtract_bit\"=false]",
        "",
        "setblock 3 -26 5 unpowered_repeater [\"minecraft:cardinal_direction\"=\"south\",\"repeater_delay\"=0]",
        "",
        "setblock 3 -26 4 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\",\"output_subtract_bit\"=true]",
        "",
        "",
        "setblock 8 -26 4 redstone_wire",
        "setblock 7 -26 4 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\",\"output_subtract_bit\"=false]",
        "setblock 6 -26 4 cauldron [\"cauldron_liquid\"=\"lava\",\"fill_level\"=6]",
        "",
        "setblock 6 -26 4 cauldron [\"cauldron_liquid\"=\"powder_snow\",\"fill_level\"=6]",
        "",
        "setblock 6 -26 4 cauldron [\"cauldron_liquid\"=\"powder_snow\",\"fill_level\"=3]",
        "",
        "setblock 6 -26 4 cauldron [\"cauldron_liquid\"=\"lava\",\"fill_level\"=3]",
        "",
        "fill 0 -26 -8 14 -24 8 air",
        "",
    ]));
    lines
}

/// A primed TNT at 8.5 -26 0.5 among eight TNT blocks two off: how long the fuse of each it lights is.
fn tnt_fuses() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["fill 5 -28 -3 11 -28 3 stone", "fill 5 -27 -3 11 -27 3 obsidian"]));
    for [x, z] in [[6, 0], [10, 0], [8, -2], [8, 2], [6, -2], [10, 2], [6, 2], [10, -2]] {
        lines.push(format!("setblock {x} -26 {z} tnt"));
    }
    lines.extend(owned(["summon tnt 8.5 -26 0.5", ""]));
    waits(&mut lines, 8);
    lines.extend(owned(["fill 5 -28 -3 11 -24 3 air", "fill 5 -27 -3 11 -27 3 stone", ""]));
    lines
}

/// A chest the actor opens with a hopper over it given ten dirt (how fast they move), then chicken loot
/// spawned on an empty hopper's top (how soon it takes fresh drops).
fn hoppers() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["setblock 3 -26 0 chest", "setblock 3 -25 0 hopper [\"facing_direction\"=0]", "setblock 6 -26 -3 hopper [\"facing_direction\"=0]", "", "!click 3 0 0 west", ""]));
    lines.extend(owned(["replaceitem block 3 -25 0 slot.container 0 dirt 10", ""]));
    waits(&mut lines, 4);
    lines.extend(owned(["!close", "loot spawn 6.5 -25 -2.5 loot \"entities/chicken\"", ""]));
    waits(&mut lines, 3);
    lines.extend(owned(["kill @e[type=item]", "fill 3 -26 -3 6 -25 0 air", ""]));
    lines
}

/// At `randomtickspeed` 100 for 25 s: twelve oak saplings on dirt (z -6 and -3, x 2 to 12), those at x 2
/// and 4 of z -6 under stone 3 up; two ripe pumpkin stems on farmland among dirt, grass, sand and farmland
/// (z 3); dry farmland 1 to 11 east of a water source (z 6). Sapling's side needs SAPLING_DRILL_WORLD=open.
fn growing() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["fill 1 -28 -7 13 -28 7 stone", "fill 1 -27 -7 13 -27 -2 dirt", "setblock 2 -23 -6 stone", "setblock 4 -23 -6 stone"]));
    for x in (2..=12).step_by(2) {
        lines.extend([format!("setblock {x} -26 -6 oak_sapling"), format!("setblock {x} -26 -3 oak_sapling")]);
    }
    for x in [3, 9] {
        lines.extend([format!("setblock {} -27 3 dirt", x - 1), format!("setblock {} -27 3 grass", x + 1), format!("setblock {x} -27 2 sand"), format!("setblock {x} -27 4 farmland")]);
        lines.extend([format!("setblock {x} -27 3 farmland [\"moisturized_amount\"=7]"), format!("setblock {x} -26 3 pumpkin_stem [\"growth\"=7]")]);
    }
    lines.extend(owned(["setblock 1 -27 6 water", "fill 2 -27 6 12 -27 6 farmland [\"moisturized_amount\"=0]", "gamerule randomtickspeed 100", ""]));
    waits(&mut lines, 25);
    lines.extend(owned(["gamerule randomtickspeed 0", "", "fill 0 -26 -8 14 -16 8 air", "fill 0 -28 -8 14 -28 8 air", "fill 0 -27 -8 14 -27 8 stone", "gamerule randomtickspeed 1", ""]));
    lines
}
