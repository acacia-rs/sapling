//! The ninth wave's block drills (research/parity/NINTH-BRIEF.md "Group 2"), on creature_drill.rs's floor
//! (stone at y -27, the stage at -26): what more blocks do before a piston (`n2pistons`), a cow in a
//! piston's way (`n2pistonmob`), what fences, panes and walls join (`n2joins`), which way a gate swings
//! (`n2gates`), a wither's T lying down (`n2wither`), a dragon egg's jumps (`n2egg`), what ores leave
//! (`n2orexp`), TNT and stairs as conductors and a torch clock by a lamp (`n2tnt`), and whether held
//! leaves keep their mark (`n2leafmark`).

use super::creatures::{floor, waits, with};

const BLOCKS: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "LevelEvent", "LevelSoundEvent", "AddItemEntity", "AddEntity", "RemoveEntity"];
const MOTION: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion"];
const Y: i32 = -26;

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "n2pistons" => (pistons(), BLOCKS),
        "n2pistonmob" => (piston_mob(), MOTION),
        "n2joins" => (joins(), BLOCKS),
        "n2gates" => (gates(), BLOCKS),
        "n2wither" => (wither(), BLOCKS),
        "n2egg" => (egg(), BLOCKS),
        "n2orexp" => (ore_xp(), BLOCKS),
        "n2tnt" => (tnt(), BLOCKS),
        "n2leafmark" => (leaf_mark(), BLOCKS),
        _ => return None,
    })
}

fn set(x: i32, y: i32, z: i32, block: &str) -> String {
    format!("setblock {x} {y} {z} {block}")
}

/// The bots out of the way at the floor's north edge.
fn aside(mut lines: Vec<String>) -> Vec<String> {
    lines.extend(["tp @s 0.5 -26 -7.5 0 0", "tp CmdWatcher 1.5 -26 -7.5", ""].map(String::from));
    lines
}

const PUSHED: [&str; 42] = [
    "bed", "wooden_door", "standing_sign", "standing_banner", "skeleton_skull [\"facing_direction\"=1]", "cake", "brewing_stand", "cauldron", "lantern", "campfire",
    "bee_nest", "composter", "sculk_sensor", "sculk_shrieker", "amethyst_cluster [\"minecraft:block_face\"=\"up\"]", "pointed_dripstone", "decorated_pot",
    "chiseled_bookshelf", "crafter", "turtle_egg", "sniffer_egg", "iron_bars", "glass_pane", "oak_fence", "cobblestone_wall", "fence_gate", "iron_chain",
    "smithing_table", "loom", "stonecutter_block", "blast_furnace", "observer", "daylight_detector", "target", "lightning_rod", "sponge", "big_dripleaf", "azalea",
    "copper_bulb", "trapped_chest", "command_block", "netherite_block",
];
const COLUMNS: [i32; 6] = [-6, -2, 2, 6, 10, 14];

/// A piston pushing east into each of [`PUSHED`], seven rows of six, a column's pistons powered at
/// once by a line of power blocks over them and let go two seconds on.
fn pistons() -> Vec<String> {
    let mut lines = aside(floor());
    for (i, block) in PUSHED.iter().enumerate() {
        let (x, z) = (COLUMNS[i % 6], -6 + 2 * (i / 6) as i32);
        lines.extend([set(x, Y, z, "piston [\"facing_direction\"=4]"), set(x + 1, Y, z, block)]);
    }
    for x in COLUMNS {
        lines.push(format!("fill {x} {} -6 {x} {} 6 redstone_block", Y + 1, Y + 1));
    }
    waits(&mut lines, 2);
    for x in COLUMNS {
        lines.push(format!("fill {x} {} -6 {x} {} 6 air", Y + 1, Y + 1));
    }
    waits(&mut lines, 2);
    lines.extend(["fill -8 -26 -6 16 -20 8 air", "kill @e[type=item]", ""].map(String::from));
    lines
}

/// Cows held by slowness: one in the cell a piston's arm goes into, one behind a stone the arm
/// pushes, one before a sticky piston, one on a piston pushing up.
fn piston_mob() -> Vec<String> {
    let mut lines = aside(floor());
    lines.extend([set(0, Y, -4, "piston [\"facing_direction\"=4]"), set(0, Y, 0, "piston [\"facing_direction\"=4]"), set(1, Y, 0, "stone")]);
    lines.extend([set(0, Y, 4, "sticky_piston [\"facing_direction\"=4]"), set(8, Y, 0, "piston [\"facing_direction\"=1]")]);
    lines.extend(["summon cow 1.5 -26 -3.5", "summon cow 2.5 -26 0.5", "summon cow 1.5 -26 4.5", "summon cow 8.5 -25 0.5", "effect @e[type=cow] slowness 60 255 true"].map(String::from));
    waits(&mut lines, 2);
    for (x, y, z) in [(0, Y + 1, -4), (0, Y + 1, 0), (0, Y + 1, 4), (7, Y, 0)] {
        lines.extend([set(x, y, z, "redstone_block"), String::new(), String::new(), set(x, y, z, "air"), String::new(), String::new()]);
    }
    lines.extend(["kill @e[type=cow]", "fill -8 -26 -6 16 -20 8 air", ""].map(String::from));
    lines
}

const JOINED: [&str; 24] = [
    "stone", "glass", "oak_leaves", "barrier", "pumpkin", "carved_pumpkin", "lit_pumpkin", "melon_block", "undyed_shulker_box", "glowstone", "slime", "honey_block",
    "tnt", "oak_stairs", "smooth_stone_slab", "chest", "fence_gate [\"minecraft:cardinal_direction\"=\"north\"]",
    "fence_gate [\"minecraft:cardinal_direction\"=\"east\"]", "spruce_fence", "nether_brick_fence", "iron_bars", "glass_pane", "cobblestone_wall", "soul_sand",
];
const JOINERS: [&str; 4] = ["oak_fence", "nether_brick_fence", "glass_pane", "cobblestone_wall"];

/// For each of [`JOINERS`], a line of it along x with each of [`JOINED`] beside it, twelve north and
/// twelve south, set one at a time after the line.
fn joins() -> Vec<String> {
    let mut lines = aside(floor());
    lines.extend(["tp @s 0.5 -26 8.5 180 0", "tp CmdWatcher 1.5 -26 8.5"].map(String::from));
    for (k, joiner) in JOINERS.iter().enumerate() {
        let z = -8 + 4 * k as i32;
        lines.push(format!("fill -8 {Y} {} 16 {Y} {0} {joiner}", z + 1));
        for (i, block) in JOINED.iter().enumerate() {
            lines.push(set(-7 + 2 * (i % 12) as i32, Y, if i < 12 { z } else { z + 2 }, block));
        }
    }
    lines.extend(["", "fill -8 -26 -8 16 -20 7 air", ""].map(String::from));
    lines
}

/// A gate east of the actor facing east, then one facing west, each clicked open and shut, with
/// the actor looking east from the west; then again with the actor looking west from the east.
fn gates() -> Vec<String> {
    let mut lines = floor();
    for (stand, click) in [("tp @s 0.5 -26 0.5 -90 0", "!click 1 0 0"), ("tp @s 2.5 -26 0.5 90 0", "!click -1 0 0")] {
        for facing in ["east", "west", "north"] {
            lines.extend([stand.to_owned(), set(1, Y, 0, &format!("fence_gate [\"minecraft:cardinal_direction\"=\"{facing}\"]")), String::new()]);
            lines.extend([click.to_owned(), String::new(), click.to_owned(), String::new()]);
        }
    }
    lines.extend([set(1, Y, 0, "air"), String::new()]);
    lines
}

/// A wither's T lying on the floor, its stem west of its arms, finished by the actor putting the last
/// skull; then the same T upright, as a check.
fn wither() -> Vec<String> {
    let mut lines = with(floor(), &["difficulty easy", "replaceitem entity @s slot.weapon.mainhand 0 wither_skeleton_skull 4"]);
    let skull = "wither_skeleton_skull [\"facing_direction\"=1]";
    lines.extend(["tp @s 6.5 -26 1.5 90 30".into(), set(2, Y, 0, "soul_sand"), "fill 3 -26 -1 3 -26 1 soul_sand".into(), set(4, Y, -1, skull), set(4, Y, 0, skull), "!place -2 -1 0".into(), String::new(), String::new()]);
    lines.extend(["kill @e[type=wither]", "fill 2 -26 -1 4 -26 1 air", ""].map(String::from));
    // The actor stands on a block so its eyes are over the arm's top.
    lines.extend([set(6, Y + 2, 1, "stone"), "tp @s 6.5 -23 1.5 90 30".into(), set(3, Y, 0, "soul_sand"), "fill 3 -25 -1 3 -25 1 soul_sand".into(), set(3, Y + 2, -1, skull), set(3, Y + 2, 0, skull), "!place -3 -2 0".into(), String::new(), String::new()]);
    lines.extend(["kill @e[type=wither]", "fill 2 -26 -1 6 -23 1 air", "clear @s", ""].map(String::from));
    lines
}

/// A dragon egg three east of the actor clicked thirty times, put back each time.
fn egg() -> Vec<String> {
    let mut lines = floor();
    for _ in 0..30 {
        lines.extend([set(3, Y, 0, "dragon_egg"), "!click 3 0 0".into()]);
    }
    lines.extend(["", "kill @e[type=falling_block]", "fill -8 -26 -8 16 -16 8 air", "kill @e[type=item]", ""].map(String::from));
    lines
}

const ORES: [&str; 8] = ["coal_ore", "diamond_ore", "emerald_ore", "lapis_ore", "redstone_ore", "quartz_ore", "nether_gold_ore", "deepslate_coal_ore"];

/// A row of each of [`ORES`], every other cell, broken by `/fill ... destroy`; then three coal ores
/// the actor breaks in survival with a diamond pickaxe.
fn ore_xp() -> Vec<String> {
    let mut lines = aside(floor());
    for (row, ore) in ORES.iter().enumerate() {
        let z = -6 + 2 * row as i32;
        lines.extend((0..12).map(|i| set(-7 + 2 * i, Y, z, ore)));
    }
    for row in 0..ORES.len() as i32 {
        let z = -6 + 2 * row;
        lines.extend([format!("fill -8 {Y} {z} 16 {Y} {z} air destroy"), String::new()]);
    }
    lines.extend(["kill @e[type=item]", "kill @e[type=xp_orb]", "tp @s 0.5 -26 8.5 180 0", "gamemode survival @s", "replaceitem entity @s slot.weapon.mainhand 0 diamond_pickaxe"].map(String::from));
    for x in [-1, 0, 1] {
        lines.extend([set(x, Y, 6, "coal_ore"), format!("!break {x} 0 -2"), String::new()]);
    }
    lines.extend(["gamemode creative @s", "clear @s", "kill @e[type=xp_orb]", "kill @e[type=item]", ""].map(String::from));
    lines
}

/// With TNT kept from going off: dust pointing into TNT and into stairs with a lamp past each; dust
/// set on TNT, on stairs and on a piston; then a torch clock (a torch on a block, six dust from it
/// back into the block) under a lamp, run ten seconds.
fn tnt() -> Vec<String> {
    let mut lines = aside(with(floor(), &["gamerule tntexplodes false"]));
    for (x, block) in [(-6, "tnt"), (-1, "oak_stairs [\"weirdo_direction\"=0]")] {
        lines.extend([set(x, Y, -4, "redstone_wire"), set(x + 1, Y, -4, block), set(x + 2, Y, -4, "redstone_lamp"), set(x - 1, Y, -4, "redstone_block"), String::new(), set(x - 1, Y, -4, "air")]);
    }
    for (x, block) in [(-6, "tnt"), (-3, "oak_stairs [\"weirdo_direction\"=0]"), (0, "piston [\"facing_direction\"=1]"), (3, "ice")] {
        lines.extend([set(x, Y, -1, block), set(x, Y + 1, -1, "redstone_wire")]);
    }
    lines.extend([String::new(), "fill -8 -26 -5 6 -24 -1 air".to_owned()]);
    lines.extend([set(8, Y, 2, "stone"), set(9, Y + 1, 2, "redstone_lamp")]);
    for (x, z) in [(10, 2), (10, 3), (10, 4), (9, 4), (8, 4), (8, 3)] {
        lines.push(set(x, Y, z, "redstone_wire"));
    }
    lines.push(set(9, Y, 2, "redstone_torch [\"torch_facing_direction\"=\"west\"]"));
    waits(&mut lines, 10);
    lines.extend(["fill 7 -26 1 11 -24 5 air", "gamerule tntexplodes true", "kill @e[type=item]", ""].map(String::from));
    lines
}

/// A log with a row of ten leaves run east from it, the far leaf's neighbour set and taken at
/// `randomTickSpeed` 200; twenty seconds on, each of the four leaves nearest the log is asked for
/// with its mark and without.
fn leaf_mark() -> Vec<String> {
    let mut lines = aside(with(floor(), &["setblock 0 -25 4 oak_log", "fill 1 -25 4 10 -25 4 oak_leaves [\"persistent_bit\"=false]", "gamerule randomtickspeed 200", ""]));
    lines.extend(["setblock 11 -25 4 oak_leaves [\"persistent_bit\"=false]", "setblock 11 -25 4 air"].map(String::from));
    waits(&mut lines, 20);
    lines.push("gamerule randomtickspeed 1".into());
    for x in 1..=5 {
        for marked in ["true", "false"] {
            lines.push(format!("testforblock {x} -25 4 oak_leaves [\"update_bit\"={marked},\"persistent_bit\"=false]"));
        }
    }
    lines.extend(["fill -8 -26 -8 16 -16 8 air", "kill @e[type=item]", ""].map(String::from));
    lines
}
