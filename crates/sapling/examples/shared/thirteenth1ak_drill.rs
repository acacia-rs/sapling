//! The thirteenth wave's group AK drills (research/parity/THIRTEENTH-BRIEF.md): a baby turtle fed
//! up (`ak1scute`), turtles bred on sand to lay (`ak1clutch`), items handed to an allay
//! (`ak1allay`), riders let off a pig by lava, campfires and ladders (`ak1exit`) and a grown
//! camel ridden (`ak1camel`); `ak1snack` eats a pack's custom food twice inside its cooldown, once after
//! (on a BDS copy with tools/ak-snack-pack.sh's pack).

use crate::creatures::{floor, waits, with};
use crate::eggs::penned;

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ak1scute" => (scute(), MOB_SHOWN),
        "ak1clutch" => (clutch(), MOB_SHOWN),
        "ak1allay" => (allay(), MOB_SHOWN),
        "ak1exit" => (exits(), EXIT_SHOWN),
        "ak1camel" => (camel(), EXIT_SHOWN),
        "ak1snack" => (snack(), SNACK_SHOWN),
        _ => return None,
    })
}

const MOB_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "TakeItemEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "UpdateBlock", "MobEquipment", "InventorySlot", "InventoryContent",
    "ContainerRegistryCleanup", "CommandOutput",
];

/// The turtle egg block of each count, set where nothing walks, so a capture names a laid egg's id.
fn egg_key(lines: &mut Vec<String>) {
    for (n, count) in ["one_egg", "two_egg", "three_egg", "four_egg"].iter().enumerate() {
        lines.push(format!("setblock {} -26 7 turtle_egg [\"turtle_egg_count\"=\"{count}\"]", 12 + n));
    }
}

/// A newborn turtle fed seagrass a second apart until it grows: the scute it drops.
fn scute() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "replaceitem entity @s slot.weapon.mainhand 0 seagrass 64", "summon turtle 2.5 -26 0.5 0 0 minecraft:entity_born", "effect @e[type=turtle] slowness 60 255 true", ""]);
    egg_key(&mut lines);
    for _ in 0..12 {
        lines.extend(["!interact turtle".to_owned(), String::new()]);
    }
    waits(&mut lines, 4);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

/// Twelve turtles on a walled sand floor, each fed seagrass once: who lays where, and how many.
fn clutch() -> Vec<String> {
    let mut lines = penned(&["fill -7 -27 -7 15 -27 7 sand", "tp CmdWatcher -6.5 -26 6.5", "replaceitem entity @s slot.weapon.mainhand 0 seagrass 64"]);
    egg_key(&mut lines);
    for (x, z) in [(1, 1), (2, 1), (5, 1), (6, 1), (9, 1), (10, 1), (1, -3), (2, -3), (5, -4), (6, -4), (9, -3), (10, -3)] {
        lines.push(format!("summon turtle {x}.5 -26 {z}.5 0 0 minecraft:spawn_adult"));
    }
    lines.extend(["".into(), "tp @s 5.5 -26 -1.5".into(), "".into(), "!feedall turtle".into()]);
    waits(&mut lines, 220);
    with(lines, &["kill @e[type=!player]", "fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -16 8 air", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

/// A held allay handed 4 dirt in survival, clicked again with the rest, with an empty hand, then
/// handed a stick, clicked sneaking, and handed a bundle that stays with it 20 s.
fn allay() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "gamemode survival @s", "replaceitem entity @s slot.weapon.mainhand 0 dirt 4", "summon allay 1.5 -26 0.5", "effect @e[type=allay] slowness 120 255 true", ""]);
    let click = |lines: &mut Vec<String>, before: &str| {
        lines.extend([before.to_owned(), "tp @e[type=allay] 1.5 -26 0.5".to_owned(), String::new(), "!interact allay".to_owned()]);
        waits(lines, 3);
    };
    click(&mut lines, "");
    click(&mut lines, "");
    click(&mut lines, "replaceitem entity @s slot.weapon.mainhand 0 air");
    click(&mut lines, "replaceitem entity @s slot.weapon.mainhand 0 stick");
    lines.extend(["!sneak".to_owned(), "!interact allay".to_owned(), String::new(), "!unsneak".to_owned()]);
    waits(&mut lines, 2);
    click(&mut lines, "replaceitem entity @s slot.weapon.mainhand 0 air");
    click(&mut lines, "replaceitem entity @s slot.weapon.mainhand 0 bundle");
    waits(&mut lines, 18);
    with(lines, &["gamemode creative @s", "kill @e[type=!player]", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

const EXIT_SHOWN: &[&str] = &[
    "AddEntity", "SetEntityLink", "SetEntityMotion", "MoveEntityDelta", "MovePlayer", "CorrectPlayerMovePrediction", "SetEntityData", "UpdateAttributes", "LevelSoundEvent", "UpdateBlock", "CommandOutput",
];

/// A slowed saddled pig ridden by `/ride` beside the blocks `around` (`(dx, dz, block)` from its
/// cell, `floor ` for one in the floor), then let off.
fn exit_trial(lines: &mut Vec<String>, x: i32, around: &[(i32, i32, &str)]) {
    lines.extend(["kill @e[type=pig]".to_owned(), format!("summon pig {x}.5 -26 -11.5 0 0 minecraft:on_saddled"), "effect @e[type=pig] slowness 30 255 true".to_owned(), String::new()]);
    lines.push("ride @s start_riding @e[type=pig,c=1] teleport_rider".to_owned());
    for (dx, dz, block) in around {
        let (y, block) = block.strip_prefix("floor ").map_or((-26, *block), |block| (-27, block));
        lines.push(format!("setblock {} {y} {} {block}", x + dx, -12 + dz));
    }
    lines.extend(["".to_owned(), "ride @s stop_riding".to_owned(), String::new(), String::new()]);
}

/// Let off with lava or a campfire to the north, with stone to the north and lava or a campfire to
/// the south, with ladders of each facing to the north, and with the north and south walled.
fn exits() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule dofiretick false", "tp CmdWatcher 18.5 -26 -6.5", "fill -2 -27 -14 24 -27 -9 stone", "fill -2 -26 -14 24 -20 -9 air", "tp @s 0.5 -26 -8.5"]);
    let trials: [&[(i32, i32, &str)]; 11] = [
        &[(0, -1, "lava")],
        &[(0, -1, "stone"), (0, 1, "lava")],
        &[(0, -1, "campfire")],
        &[(0, -1, "stone"), (0, 1, "campfire")],
        &[(0, -1, "ladder [\"facing_direction\"=2]")],
        &[(0, -1, "ladder [\"facing_direction\"=3]")],
        &[(0, -1, "ladder [\"facing_direction\"=4]")],
        &[(0, -1, "ladder [\"facing_direction\"=5]")],
        &[(0, -1, "stone"), (0, 1, "stone")],
        &[(0, -1, "fire")],
        &[(0, -1, "floor magma")],
    ];
    for (n, around) in trials.iter().enumerate() {
        // Positive cells only: `{x}.5` is the wrong cell for a negative x.
        exit_trial(&mut lines, 1 + 2 * n as i32, around);
        lines.push("fill -2 -26 -14 24 -20 -9 air".to_owned());
        lines.push("fill -2 -27 -14 24 -27 -9 stone".to_owned());
    }
    with(lines, &["kill @e[type=!player]", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

/// A grown camel saddled by `/replaceitem`, ridden south 3 s, its dash charged 10 ticks, let off.
fn camel() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 8.5 -26 -7.5", "tp @s 0.5 -26 -7.5 0 0", ""]);
    lines.extend(["summon camel 0.5 -26 -6.5 0 0 minecraft:spawn_adult", "replaceitem entity @e[type=camel] slot.saddle 0 saddle", "!mount camel", "", "!look 0 1.6 8", "!walk", "", "", "", "!stop", "", "!charge 10", "", "", "!dismount", ""].map(String::from));
    with(lines, &["kill @e[type=!player]", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

const SNACK_SHOWN: &[&str] = &["UpdateAttributes", "ClientStartItemCooldown", "CompletedUsingItem", "InventorySlot", "InventoryContent", "EntityEvent", "LevelSoundEvent", "CommandOutput"];

/// Starved, then `ak:snack` (4 hunger, 0.6 saturation, 1.6 s, a 3 s `ak_snack` cooldown) eaten, again
/// 0.4 s after the first bite ends, and again after the cooldown.
fn snack() -> Vec<String> {
    let mut lines = with(floor(), &["gamemode survival @s", "effect @s hunger 3 255 true", "", "", "", "effect @s clear", "replaceitem entity @s slot.weapon.mainhand 0 ak:snack 4", ""]);
    lines.extend(["!eat", "", "", "!eat", "", "", "", "", "!eat", "", "", ""].map(String::from));
    with(lines, &["gamemode creative @s", "clear @s", "effect @s clear", ""])
}
