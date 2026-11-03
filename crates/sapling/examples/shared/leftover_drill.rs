//! Drills of the follow-up wave's blocks and effects (docs/research redstone.md, blocks.md,
//! food-effects.md, effects.md, splash-sound.md, mob-damage.md): blocks set where nothing holds
//! them, a torch beside a block slime moves, big dripleaves stood on, a dispensed splash potion's
//! shares, a viewer joining after effects were given, bodies entering water, rain on the drill
//! floor. On creature_drill.rs's floor (stone at y -27, the bots at y -26).

use super::creatures::{floor, waits, with};

/// Packets shown of these drills.
pub const LEFTOVER_SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "LevelEvent", "LevelSoundEvent", "AddItemEntity", "AddEntity", "AddPlayer", "RemoveEntity", "MobEffect", "SetEntityData", "EntityEvent",
    "UpdateAttributes",
];
/// [`LEFTOVER_SHOWN`] with every move, for the drills that measure where bodies were.
const MOVES_SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateSubchunkBlocks", "LevelEvent", "LevelSoundEvent", "AddEntity", "RemoveEntity", "MobEffect", "EntityEvent", "MovePlayer", "MoveEntityAbsolute", "MoveEntityDelta",
    "SetEntityMotion", "SetEntityData",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "unheld" => (unheld(), LEFTOVER_SHOWN),
        "leafstand" => (leaf_stand(), MOVES_SHOWN),
        "splashshare" => (splash_share(), MOVES_SHOWN),
        "lateeffects" => (late_effects(), LEFTOVER_SHOWN),
        "wetflags" => (wet_flags(), LEFTOVER_SHOWN),
        "rainvoid" => (rain_void(), LEFTOVER_SHOWN),
        _ => return None,
    })
}

fn set(x: i32, y: i32, z: i32, block: &str) -> String {
    format!("setblock {x} {y} {z} {block}")
}

/// Blocks set over air with nothing to hang or stand on, one a line, then a supported wall torch;
/// a piston pushing slime up with a stone beside it and a torch on that stone's far side.
fn unheld() -> Vec<String> {
    let mut lines = floor();
    let blocks = [
        "torch [\"torch_facing_direction\"=\"west\"]",
        "torch [\"torch_facing_direction\"=\"top\"]",
        "redstone_torch",
        "lever [\"lever_direction\"=\"up_north_south\"]",
        "stone_button [\"facing_direction\"=1]",
        "ladder [\"facing_direction\"=3]",
        "poppy",
        "rail",
        "white_carpet",
        "wall_sign [\"facing_direction\"=3]",
    ];
    for (i, block) in blocks.iter().enumerate() {
        lines.extend([set(-6 + 2 * i as i32, -24, -6, block), String::new()]);
    }
    lines.extend([set(13, -24, -4, "stone"), set(14, -24, -4, "torch [\"torch_facing_direction\"=\"west\"]"), String::new()]);
    // Slime at (4, -25, 2) on a piston pushing up; the stone west of it goes along, the torch on its
    // west face (`east`: facing.rs) stays in its cell.
    lines.extend([set(4, -26, 2, "piston [\"facing_direction\"=1]"), set(4, -25, 2, "slime"), set(3, -25, 2, "stone"), set(2, -25, 2, "torch [\"torch_facing_direction\"=\"east\"]"), String::new()]);
    lines.extend([set(5, -26, 2, "redstone_block"), String::new(), String::new(), "kill @e[type=item]".to_owned()]);
    lines
}

/// The actor on a big dripleaf on dirt for 12 s; on a leaf over its stem for 12 s; a cow on a third.
/// The watcher sees the actor's moves.
fn leaf_stand() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "setblock 2 -27 2 dirt", "setblock 2 -26 2 big_dripleaf [\"big_dripleaf_head\"=true]", ""]);
    lines.extend(["tp @s 2.5 -25 2.5".to_owned()]);
    waits(&mut lines, 12);
    lines.extend(with(Vec::new(), &["tp @s 0.5 -26 -3.5", "setblock 6 -27 2 dirt", "setblock 6 -26 2 big_dripleaf [\"big_dripleaf_head\"=false]", "setblock 6 -25 2 big_dripleaf [\"big_dripleaf_head\"=true]", "tp @s 6.5 -24 2.5"]));
    waits(&mut lines, 12);
    lines.extend(with(Vec::new(), &["tp @s 0.5 -26 -3.5", "setblock 10 -27 2 dirt", "setblock 10 -26 2 big_dripleaf [\"big_dripleaf_head\"=true]", "summon cow 10.5 -25 2.5", "effect @e[type=cow] slowness 60 255 true"]));
    waits(&mut lines, 12);
    lines.push("kill @e[type=cow]".to_owned());
    lines
}

/// A dispenser at (0, -26, 4) facing east shoots splash potions of poison at the actor 3 east of
/// it, the watcher 2.1 south of the actor, then 2.1 east (behind it), then 1.5 north; last one of
/// harming. Both in survival so the potions take.
fn splash_share() -> Vec<String> {
    let mut lines = with(floor(), &["setblock 0 -26 4 dispenser [\"facing_direction\"=5]", "gamemode survival @s", "gamemode survival CmdWatcher", "tp @s 3.5 -26 4.5 -90 0"]);
    for (watcher, potion) in [("3.5 -26 6.6", "25"), ("5.6 -26 4.5", "25"), ("3.5 -26 3", "25"), ("3.5 -26 6.6", "23")] {
        lines.extend([format!("tp CmdWatcher {watcher}"), format!("replaceitem block 0 -26 4 slot.container 0 splash_potion 1 {potion}"), set(-1, -26, 4, "redstone_block"), String::new(), String::new()]);
        lines.extend(with(Vec::new(), &["setblock -1 -26 4 air", "effect @a clear", "effect @a instant_health 1 4 true", ""]));
    }
    lines.extend(with(Vec::new(), &["gamemode creative @s", "gamemode creative CmdWatcher"]));
    lines
}

/// A cow given speed and slowness and the watcher given jump boost; then the actor leaves and
/// joins again, and is told of both.
fn late_effects() -> Vec<String> {
    let mut lines = with(floor(), &["summon cow 3.5 -26 0.5", "effect @e[type=cow] speed 60 1", "effect @e[type=cow] slowness 60 255 true", "effect CmdWatcher jump_boost 60 0", ""]);
    lines.extend(["!rejoin".to_owned(), String::new(), String::new(), String::new()]);
    lines.extend(with(Vec::new(), &["kill @e[type=cow]", "effect CmdWatcher clear", ""]));
    lines
}

/// A pool 3 deep at x 3-5, z 3-5: a cow dropped in, a zombie dropped in, the actor put in and out.
fn wet_flags() -> Vec<String> {
    let mut lines = with(floor(), &["fill 2 -27 2 6 -24 6 stone", "fill 3 -26 3 5 -24 5 water", "", "summon cow 4.5 -21 4.5"]);
    waits(&mut lines, 4);
    lines.extend(["kill @e[type=cow]".to_owned(), "summon zombie 4.5 -21 4.5".to_owned()]);
    waits(&mut lines, 4);
    lines.extend(with(Vec::new(), &["kill @e[type=zombie]", "tp @s 4.5 -22 4.5", "", "", "", "tp @s 0.5 -26 0.5", "", "fill 2 -27 2 6 -24 6 air", "fill 2 -27 2 6 -27 6 stone"]));
    lines
}

/// Rain on the drill floor: an enderman and a burning cow under open sky.
fn rain_void() -> Vec<String> {
    let mut lines = with(floor(), &["summon enderman 4.5 -26 4.5", "summon cow 6.5 -26 0.5", "effect @e[type=!player] slowness 60 255 true", "setblock 6 -26 0 fire", "", "setblock 6 -26 0 air", "weather rain"]);
    waits(&mut lines, 8);
    lines.extend(with(Vec::new(), &["weather clear", "kill @e[type=!player]", ""]));
    lines
}
