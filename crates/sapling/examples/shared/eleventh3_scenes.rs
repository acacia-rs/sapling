//! The lines of eleventh3_drill.rs's scenes. Cells are absolute; `!` offsets count from the actor's
//! feet, at 10 -60 10 on redstone_parts_drill.rs's stage, or 0.5 -26 0.5 on creature_drill.rs's floor.

use crate::creatures::{floor, waits, with};
use crate::redstone_parts::{set, stage};

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

/// 200 lit redstone ores in rows z 40 to 44 at `randomtickspeed` 1 for 90 s, 20 for 10 s, then 100 for
/// 60 s with 28 lone blocks of copper six apart; each ore's UpdateBlock times its life.
pub fn pace() -> Vec<String> {
    let mut lines = stage([0, 44], [0, 44]);
    lines.push("gamerule randomtickspeed 0".into());
    for (rule, seconds, copper) in [(1, 90, false), (20, 10, false), (100, 60, true)] {
        lines.push("fill 2 -60 40 41 -60 44 lit_redstone_ore".into());
        if copper {
            for x in (2..=38).step_by(6) {
                for z in (14..=32).step_by(6) {
                    lines.push(set(x, -60, z, "copper_block"));
                }
            }
        }
        lines.extend([format!("gamerule randomtickspeed {rule}"), String::new()]);
        waits(&mut lines, seconds);
        lines.extend(owned(["gamerule randomtickspeed 0", "fill 2 -60 14 41 -60 44 air", ""]));
    }
    with(lines, &["gamerule randomtickspeed 1"])
}

/// Two brown mushrooms on stone sealed in stone at `randomtickspeed` 0: a light block set beside each
/// (13 and 12 in the mushroom's cell), then a block over each changed twice. Then seven rooms a light
/// level of grass beside dirt, the grass 8 to 11 over its top, the dirt one less, for 60 s at 200.
pub fn light() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule randomtickspeed 0", "fill -7 -28 -7 15 -20 7 stone", "tp @s 0.5 -19 0.5", "tp CmdWatcher 0.5 -19 0.5", ""]);
    lines.extend(owned(["setblock 10 -21 -6 brown_mushroom", "setblock 13 -21 -6 brown_mushroom", ""]));
    lines.extend(owned(["setblock 11 -21 -6 light_block_14", "setblock 14 -21 -6 light_block_13", "", ""]));
    for block in ["cobblestone", "stone"] {
        lines.extend([set(10, -20, -6, block), set(13, -20, -6, block), String::new(), String::new()]);
    }
    for (row, light) in (9..=12).enumerate() {
        let z = -6 + 4 * row as i32;
        for k in 0..7 {
            let x = -6 + 3 * k;
            lines.push(format!("fill {x} -26 {z} {} -24 {z} air", x + 1));
            lines.extend([set(x, -27, z, "grass_block"), set(x + 1, -27, z, "dirt"), set(x, -25, z, &format!("light_block_{light}"))]);
        }
    }
    lines.extend(owned(["", "gamerule randomtickspeed 200"]));
    waits(&mut lines, 60);
    with(lines, &["gamerule randomtickspeed 1", "fill -7 -28 -7 15 -20 7 air", "fill -8 -27 -8 16 -27 8 stone", "kill @e[type=item]", ""])
}

/// Torches and lamps on the floor: a torch on a stone under a lamp, and one feeding three dust to a
/// lamp, each stone powered by a block of redstone beside it for 2 s and let go; then a torch clock
/// (a torch on a stone's side, six dust back into the stone) under a lamp, a lamp under its second
/// dust, run 10 s.
pub fn lamps() -> Vec<String> {
    let y = -26;
    let mut lines = with(floor(), &["tp @s 0.5 -26 -7.5 0 0", "tp CmdWatcher 1.5 -26 -7.5", ""]);
    lines.extend([set(0, y, 0, "stone"), set(0, y + 1, 0, "redstone_torch"), set(0, y + 2, 0, "redstone_lamp")]);
    lines.extend([format!("fill 4 {y} 0 8 {y} 0 stone"), set(4, y + 1, 0, "redstone_torch"), format!("fill 5 {} 0 7 {} 0 redstone_wire", y + 1, y + 1), set(8, y + 1, 0, "redstone_lamp")]);
    lines.extend([String::new(), String::new()]);
    for x in [0, 4] {
        lines.extend([set(x, y, -1, "redstone_block"), String::new(), String::new(), set(x, y, -1, "air"), String::new(), String::new()]);
    }
    lines.extend([set(8, y, 4, "stone"), set(9, y + 1, 4, "redstone_lamp"), set(10, y - 1, 5, "redstone_lamp")]);
    for (x, z) in [(10, 4), (10, 5), (10, 6), (9, 6), (8, 6), (8, 5)] {
        lines.push(set(x, y, z, "redstone_wire"));
    }
    lines.push(set(9, y, 4, "redstone_torch [\"torch_facing_direction\"=\"west\"]"));
    waits(&mut lines, 10);
    with(lines, &["fill -2 -26 -2 12 -23 8 air", "setblock 10 -27 5 stone", ""])
}

/// The bots at the floor's north edge; pistons pushing a stone east with a stand before it, an arm
/// into a stand, a sticky arm into a stand half in its cell, and an up piston under a stand; twice.
pub fn piston_mob() -> Vec<String> {
    let y = -26;
    let mut lines = with(floor(), &["tp @s 0.5 -26 -7.5 0 0", "tp CmdWatcher 1.5 -26 -7.5", ""]);
    for _ in 0..2 {
        lines.extend([set(0, y, -4, "piston [\"facing_direction\"=4]"), set(0, y, 0, "piston [\"facing_direction\"=4]"), set(1, y, 0, "stone")]);
        lines.extend([set(0, y, 4, "sticky_piston [\"facing_direction\"=4]"), set(8, y, 0, "piston [\"facing_direction\"=1]")]);
        lines.extend(owned(["summon armor_stand 1.5 -26 -3.5", "summon armor_stand 2.5 -26 0.5", "summon armor_stand 1.5 -26 4.5", "summon armor_stand 8.5 -25 0.5", "", ""]));
        for (x, y, z) in [(0, y + 1, -4), (0, y + 1, 0), (0, y + 1, 4), (7, y, 0)] {
            lines.extend([set(x, y, z, "redstone_block"), String::new(), String::new(), set(x, y, z, "air"), String::new(), String::new()]);
        }
        lines.extend(owned(["kill @e[type=armor_stand]", "fill -8 -26 -6 16 -20 8 air", ""]));
    }
    lines
}

/// At noon: a daylight detector set; a sculk sensor, then a bed set near it and set to air; a frame
/// set with nothing behind it and one on a stone, then each set to air.
pub fn sets() -> Vec<String> {
    let mut lines = stage([0, 20], [0, 20]);
    lines.extend(owned(["time set noon", "setblock 4 -60 4 daylight_detector", "", "", "", ""]));
    lines.extend(owned(["setblock 2 -60 14 sculk_sensor", "", "", "setblock 0 -60 17 bed", "", "", "", "setblock 0 -60 17 air", "", "", ""]));
    lines.extend(owned(["setblock 6 -60 6 frame", "", "", "setblock 5 -60 8 stone", "setblock 6 -60 8 frame", "", "", "setblock 6 -60 6 air", "", "setblock 6 -60 8 air", ""]));
    with(lines, &["kill @e[type=item]", "fill 0 -60 0 20 -60 20 air", ""])
}

/// Coral out of water: four plants set on the stage, four fans and four wall fans put by hand on
/// stones about the actor; watched 10 s.
pub fn coral() -> Vec<String> {
    let mut lines = stage([0, 20], [0, 20]);
    for x in [3, 5, 15, 17] {
        lines.push(set(x, -60, 16, "tube_coral"));
    }
    for (dx, dz) in [(2, -2), (2, 2), (-2, 2), (-2, -2)] {
        lines.push(set(10 + dx, -60, 10 + dz, "stone"));
    }
    lines.extend([hold("tube_coral_fan 8"), String::new()]);
    for (dx, dz, side) in [(2, -2, "east"), (2, 2, "east"), (-2, 2, "west"), (-2, -2, "west")] {
        lines.extend([format!("!place {dx} 0 {dz} up"), format!("!place {dx} 0 {dz} {side}")]);
    }
    waits(&mut lines, 10);
    with(lines, &["clear @s", "fill 0 -60 0 20 -59 20 air", ""])
}

/// Forty dry coral plants filled at once, for the spread of their waits, and a row of filled fans.
pub fn coral_many() -> Vec<String> {
    let mut lines = stage([0, 20], [0, 20]);
    lines.extend(owned(["fill 2 -60 2 11 -60 5 tube_coral", "fill 2 -60 8 11 -60 8 brain_coral_fan"]));
    waits(&mut lines, 10);
    with(lines, &["fill 0 -60 0 20 -59 20 air", ""])
}

/// Creative, the bot's input as a mouse's (probe.rs): a frame holding a diamond broken, then an
/// empty frame broken, each on a stone's west side four blocks east, facing the bot.
pub fn frame() -> Vec<String> {
    let mut lines = stage([0, 20], [0, 20]);
    for z in [10, 12] {
        lines.extend([set(15, -60, z, "stone"), set(14, -60, z, "frame [\"facing_direction\"=4]")]);
    }
    lines.extend([hold("diamond 3"), "!click 4 0 0 west".into(), String::new(), "clear @s".into(), String::new()]);
    lines.extend(owned(["!break 4 0 0", "", "", "!break 4 0 2", "", "", "kill @e[type=item]", "fill 0 -60 0 20 -59 20 air", ""]));
    lines
}

/// A command block minecart given `say cart` with its output tracked, run on a powered activator
/// rail for 2 s and left on it unpowered, for the world folder's actor to be read.
pub fn cart() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.extend(owned(["gamerule commandblocksenabled true", "gamerule commandblockoutput true", "setblock 15 -60 10 activator_rail", "summon command_block_minecart 15.5 -60 10.5", "", ""]));
    lines.extend(owned(["!cartcmd say cart", "", "setblock 15 -61 10 redstone_block", "", "", "setblock 15 -61 10 stone", "", "", ""]));
    lines
}

/// An 8x1x8 stone floor saved, then loaded by a structure block at integrity 50 under seeds 0, 0
/// again, 1, 12345 and 4294967289.
pub fn seed() -> Vec<String> {
    let mut lines = stage([0, 20], [0, 70]);
    lines.extend(owned(["fill 2 -60 2 9 -60 9 stone", "structure save e3seed 2 -60 2 9 -60 9", "fill 2 -60 2 9 -60 9 air", ""]));
    for (i, seed) in ["0", "0", "1", "12345", "4294967289"].iter().enumerate() {
        let z = 12 + 10 * i as i32;
        // BDS ignored an update from the bot 11 blocks away, and a first one that also triggered.
        lines.extend([format!("tp @s 0.5 -60 {}.5", z - 2), set(1, -60, z, "structure_block [\"structure_block_type\"=\"load\"]"), String::new()]);
        // A block looks a name up as typed; `/structure save` kept `mystructure:e3seed` (structure-blocks.md).
        lines.extend([format!("!e3sb 1 -60 {z} mystructure:e3seed 50 {seed} 0"), String::new(), format!("!e3sb 1 -60 {z} mystructure:e3seed 50 {seed} 1"), String::new(), String::new()]);
    }
    with(lines, &["fill 1 -60 12 9 -60 60 air", "structure delete e3seed", ""])
}

/// 64 blocks with turning states, each walled in stone two apart in rows z 2 and 4, saved and loaded
/// at 90, 270, 90 mirrored on x, 180, and 0 mirrored on x and on z.
const TURNED: [&str; 64] = [
    "lever [\"lever_direction\"=\"north\"]", "lever [\"lever_direction\"=\"up_north_south\"]", "lever [\"lever_direction\"=\"down_east_west\"]", "lever [\"lever_direction\"=\"east\"]",
    "rail [\"rail_direction\"=6]", "rail [\"rail_direction\"=9]", "rail [\"rail_direction\"=2]", "golden_rail [\"rail_direction\"=1]",
    "activator_rail [\"rail_direction\"=4]", "vine [\"vine_direction_bits\"=3]", "glow_lichen [\"multi_face_direction_bits\"=6]", "sculk_vein [\"multi_face_direction_bits\"=33]",
    "resin_clump [\"multi_face_direction_bits\"=20]", "standing_sign [\"ground_sign_direction\"=3]",
    "oak_hanging_sign [\"facing_direction\"=5,\"ground_sign_direction\"=6,\"hanging\"=true,\"attached_bit\"=true]",
    "oak_stairs [\"weirdo_direction\"=0,\"minecraft:corner\"=\"inner_left\"]", "oak_stairs [\"weirdo_direction\"=2,\"minecraft:corner\"=\"outer_right\",\"upside_down_bit\"=true]",
    "trapdoor [\"direction\"=1,\"open_bit\"=true]", "fence_gate [\"minecraft:cardinal_direction\"=\"east\"]", "unpowered_repeater [\"minecraft:cardinal_direction\"=\"east\"]",
    "unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\"]", "observer [\"minecraft:facing_direction\"=\"east\"]", "observer [\"minecraft:facing_direction\"=\"up\"]",
    "hopper [\"facing_direction\"=2]", "wooden_button [\"facing_direction\"=4]", "ladder [\"facing_direction\"=2]", "redstone_torch [\"torch_facing_direction\"=\"north\"]",
    "torch [\"torch_facing_direction\"=\"east\"]", "end_rod [\"facing_direction\"=5]", "grindstone [\"attachment\"=\"side\",\"direction\"=1]",
    "bell [\"attachment\"=\"multiple\",\"direction\"=1]", "skeleton_skull [\"facing_direction\"=2]",
    "big_dripleaf [\"minecraft:cardinal_direction\"=\"east\"]", "small_dripleaf_block [\"minecraft:cardinal_direction\"=\"north\",\"upper_block_bit\"=false]",
    "pink_petals [\"minecraft:cardinal_direction\"=\"west\",\"growth\"=2]", "leaf_litter [\"minecraft:cardinal_direction\"=\"east\",\"growth\"=1]", "chiseled_bookshelf [\"direction\"=1]",
    "campfire [\"minecraft:cardinal_direction\"=\"east\"]", "lectern [\"minecraft:cardinal_direction\"=\"east\"]", "anvil [\"minecraft:cardinal_direction\"=\"east\"]",
    "end_portal_frame [\"minecraft:cardinal_direction\"=\"west\"]", "cocoa [\"direction\"=1]", "tripwire_hook [\"direction\"=1]", "carved_pumpkin [\"minecraft:cardinal_direction\"=\"east\"]",
    "dispenser [\"facing_direction\"=5]", "piston [\"facing_direction\"=5]", "sticky_piston [\"facing_direction\"=2]", "wall_banner [\"facing_direction\"=5]", "wall_sign [\"facing_direction\"=4]",
    "decorated_pot [\"direction\"=1]", "calibrated_sculk_sensor [\"minecraft:cardinal_direction\"=\"east\"]", "stonecutter_block [\"minecraft:cardinal_direction\"=\"east\"]",
    "loom [\"direction\"=1]", "beehive [\"direction\"=3]", "oak_log [\"pillar_axis\"=\"x\"]", "iron_chain [\"pillar_axis\"=\"z\"]",
    "cobblestone_wall [\"wall_connection_type_east\"=\"short\",\"wall_connection_type_north\"=\"tall\",\"wall_post_bit\"=true]",
    "oak_fence [\"minecraft:connection_east\"=true,\"minecraft:connection_south\"=true]", "jigsaw [\"facing_direction\"=5,\"rotation\"=1]", "jigsaw [\"facing_direction\"=1,\"rotation\"=2]",
    "barrel [\"facing_direction\"=5]", "oak_shelf [\"minecraft:cardinal_direction\"=\"east\"]", "frame [\"facing_direction\"=4]", "vault [\"minecraft:cardinal_direction\"=\"east\"]",
];

pub fn turn() -> Vec<String> {
    let mut lines = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamemode creative @s", "clear @s", "fill 0 -61 -24 66 -61 74 stone"]);
    lines.extend(owned(["fill 0 -60 -24 66 -55 24 air", "fill 0 -60 25 66 -55 74 air", "tp @s 10.5 -60 10.5", "fill 1 -61 1 65 -59 5 stone", ""]));
    for (at, block) in TURNED.iter().enumerate() {
        lines.push(set(2 + 2 * (at % 32) as i32, -60, if at < 32 { 2 } else { 4 }, block));
    }
    lines.extend(owned(["", "structure save e3turn 2 -60 2 64 -60 4", ""]));
    let loads = [("20 -60 10", "90_degrees", "none"), ("26 -60 10", "270_degrees", "none"), ("32 -60 10", "90_degrees", "x"), ("2 -60 -10", "180_degrees", "none"), ("2 -60 -16", "0_degrees", "x"), ("2 -60 -22", "0_degrees", "z")];
    for (at, rotation, mirror) in loads {
        lines.extend([format!("structure load e3turn {at} {rotation} {mirror}"), String::new(), String::new()]);
    }
    lines.extend(owned(["kill @e[type=item]", "fill 0 -60 -24 66 -55 24 air", "fill 0 -60 25 66 -55 74 air", "fill 0 -59 -24 66 -59 74 air", "structure delete e3turn", ""]));
    lines
}
