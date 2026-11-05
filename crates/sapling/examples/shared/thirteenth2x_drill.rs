//! The thirteenth wave's group X drills (research/parity/THIRTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27, the actor at 0.5 -26 0.5 facing east; clicks are offsets from its cell). `x2clone`: stone,
//! two cells, a waterlogged fence, a chest and a moved shulker box cloned. `x2ran`: a vault opened with a key and a
//! trial spawner whose wave ran (mob spawning on, which a trial spawner needs), each cloned. `x2bed`: a bed's head
//! clicked in the Nether. `x2boom`: TNT and a forced creeper on the floor. `x2drink`: a water bottle, honey and
//! milk drunk beside a sculk sensor.

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "BlockEvent", "LevelEvent", "LevelEventGeneric", "LevelSoundEvent", "AddEntity", "AddItemEntity", "RemoveEntity",
    "UpdateAttributes", "EntityEvent", "SetHealth", "CommandOutput",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    let lines = match name {
        "x2clone" => clone(),
        "x2ran" => ran(),
        "x2bed" => bed(),
        "x2boom" => boom(),
        "x2drink" => drink(),
        _ => return None,
    };
    Some((lines, SHOWN))
}

fn cloned(lines: Vec<String>, line: &str) -> Vec<String> {
    with(lines, &[line, "", ""])
}

/// Stone into air and onto stone, two cobblestones, a fence set over water in the floor, a chest with stacks,
/// and a shulker box with stacks moved.
fn clone() -> Vec<String> {
    let mut lines = with(floor(), &["setblock 2 -26 -3 stone", ""]);
    lines = cloned(lines, "clone 2 -26 -3 2 -26 -3 4 -26 -3");
    lines = cloned(lines, "clone 2 -26 -3 2 -26 -3 4 -26 -3");
    lines = with(lines, &["fill 2 -26 -1 3 -26 -1 cobblestone", ""]);
    lines = cloned(lines, "clone 2 -26 -1 3 -26 -1 5 -26 -1");
    lines = with(lines, &["setblock 2 -27 5 water", "", "setblock 2 -27 5 oak_fence", ""]);
    lines = cloned(lines, "clone 2 -27 5 2 -27 5 4 -27 5");
    lines = with(lines, &["setblock 2 -26 3 chest", "replaceitem block 2 -26 3 slot.container 0 stone 5", ""]);
    lines = cloned(lines, "clone 2 -26 3 2 -26 3 4 -26 3");
    lines = with(lines, &["setblock 7 -26 -3 undyed_shulker_box", "replaceitem block 7 -26 -3 slot.container 0 apple 2", ""]);
    lines = cloned(lines, "clone 7 -26 -3 7 -26 -3 9 -26 -3 replace move");
    with(lines, &["fill 2 -26 -6 12 -26 6 air", "fill 2 -27 -6 12 -27 6 stone", ""])
}

/// A vault in reach opened by a survival player with a trial key, then cloned; a trial spawner given zombies
/// whose wave is killed as it comes, left to eject, then cloned.
fn ran() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule domobspawning true", "setblock 2 -26 -2 vault", "gamemode survival @s", "replaceitem entity @s slot.weapon.mainhand 0 trial_key 1", "", "", "!click 2 0 -2 west"]);
    waits(&mut lines, 5);
    lines = with(lines, &["gamemode creative @s", "kill @e[type=item]", ""]);
    lines = cloned(lines, "clone 2 -26 -2 2 -26 -2 5 -26 -2");
    lines = with(lines, &["setblock 3 -26 3 trial_spawner", "replaceitem entity @s slot.weapon.mainhand 0 zombie_spawn_egg 1", "", "!click 3 0 3 west", "", "gamemode survival @s"]);
    for _ in 0..30 {
        lines = with(lines, &["", "kill @e[type=zombie]"]);
    }
    waits(&mut lines, 4);
    lines = with(lines, &["gamemode creative @s", "kill @e[type=item]", ""]);
    lines = cloned(lines, "clone 3 -26 3 3 -26 3 6 -26 3");
    with(lines, &["gamerule domobspawning false", "fill 2 -26 -6 12 -26 6 air", ""])
}

/// The actor in the Nether first, then its floor laid, a bed set and its head clicked.
fn bed() -> Vec<String> {
    let mut lines = with(floor(), &["execute in nether run tp @s 0.5 100 0.5 -90 0"]);
    waits(&mut lines, 10);
    lines = with(lines, &["fill -6 99 -6 12 99 12 obsidian", "fill -6 100 -6 12 106 12 air", "tp @s 0.5 100 0.5 -90 0", "", "setblock 0 100 4 bed [\"direction\"=0]", "", "!click 0 0 4", ""]);
    waits(&mut lines, 2);
    with(lines, &["execute in overworld run tp @s 0.5 -26 0.5 -90 0", "kill @e[type=item]", ""])
}

/// Primed TNT, then a creeper made to burst, each on the floor a few blocks off.
fn boom() -> Vec<String> {
    let mut lines = with(floor(), &["summon tnt 5.5 -26 0.5"]);
    waits(&mut lines, 6);
    lines = with(lines, &["kill @e[type=item]", "fill -8 -27 -8 16 -27 8 stone", "", "summon creeper 5.5 -26 -3.5 0 0 minecraft:start_exploding_forced"]);
    waits(&mut lines, 5);
    with(lines, &["kill @e[type=item]", "fill -8 -27 -8 16 -27 8 stone", ""])
}

/// A sensor at 6 -25 4 on stone; a survival actor drinks a water bottle, honey and milk, a while apart.
fn drink() -> Vec<String> {
    let mut lines = with(floor(), &["setblock 6 -26 4 stone", "setblock 6 -25 4 sculk_sensor", "gamemode survival @s"]);
    waits(&mut lines, 3);
    for drink in ["potion", "honey_bottle", "milk_bucket"] {
        lines = with(lines, &[&format!("replaceitem entity @s slot.weapon.mainhand 0 {drink} 1"), "", "!eat"]);
        waits(&mut lines, 5);
    }
    with(lines, &["gamemode creative @s", "fill 6 -26 4 6 -25 4 air", ""])
}
