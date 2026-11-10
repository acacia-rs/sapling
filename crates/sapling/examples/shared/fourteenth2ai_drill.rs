//! Fourteenth wave, group AI (drill2): `ai2kin`, babies (lamb, piglet) 8.5, 9.75 and 10.5 east of a
//! held grown one of their kind, each pair 20 apart along z: which take it up (BDS's
//! `FollowParentGoal` search box, read under gdb). `ai2push`, three zombies crowded into a ladder,
//! vine and scaffolding cell walled on three sides, and a lone zombie in an open ladder cell knocked
//! into it from the west, a sculk sensor four west of each: do pushed mobs climb, and are their
//! climbs heard.

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn field() -> Vec<String> {
    owned([
        "kill @e[type=!player]",
        "gamerule domobspawning false",
        "gamerule dodaylightcycle false",
        "gamerule falldamage false",
        "weather clear",
        "difficulty easy",
        "time set 18000",
        "gamemode creative @a",
        "effect @a clear",
        "clear @s",
        "tp CmdWatcher 0.5 -60 -90.5",
        "",
    ])
}

/// Kind, its adult spawn event, and the baby's offset east.
const PAIRS: &[(&str, &str, f32)] = &[
    ("sheep", "spawn_adult", 8.5),
    ("sheep", "spawn_adult", 9.75),
    ("sheep", "spawn_adult", 10.5),
    ("pig", "minecraft:spawn_adult", 8.5),
    ("pig", "minecraft:spawn_adult", 9.75),
    ("pig", "minecraft:spawn_adult", 10.5),
];

fn kin() -> Vec<String> {
    let mut out = field();
    for z in [-60, -30, 0, 30] {
        out.push(format!("fill -4 -61 {z} 16 -61 {} grass", z + 29));
        out.push(format!("fill -4 -60 {z} 16 -55 {} air", z + 29));
    }
    out.extend(owned(["tp @s -20.5 -60 0.5", ""]));
    for (row, (kind, adult, _)) in PAIRS.iter().enumerate() {
        let z = -50 + 20 * row as i32;
        out.push(format!("summon {kind} 0.5 -60 {z}.5 0 0 {adult} mum{row}"));
    }
    out.extend(owned(["effect @e[type=sheep] slowness 1000 255 true", "effect @e[type=pig] slowness 1000 255 true", ""]));
    wait(&mut out, 2);
    for (row, (kind, _, off)) in PAIRS.iter().enumerate() {
        let z = -50 + 20 * row as i32;
        out.push(format!("summon {kind} {} -60 {z}.5 270 0 minecraft:entity_born", 0.5 + off));
    }
    wait(&mut out, 12);
    out.extend(owned(["kill @e[type=!player]", ""]));
    out
}

/// A cell at x 2 of row `z`: walled west, north and south when `walled`, stone east (x 3), and a
/// sculk sensor at x -2.
fn cell(out: &mut Vec<String>, z: i32, climbable: &str, walled: bool) {
    out.push(format!("fill 3 -60 {} 3 -55 {} stone", z - 1, z + 1));
    if walled {
        out.push(format!("fill 1 -60 {} 2 -55 {} stone", z - 1, z + 1));
    }
    out.push(format!("fill 2 -60 {z} 2 -55 {z} {climbable}"));
    out.push(format!("setblock -2 -60 {z} sculk_sensor"));
}

fn push() -> Vec<String> {
    let mut out = field();
    out.extend(owned(["fill -6 -61 -12 6 -61 12 stone", "fill -6 -60 -12 6 -54 12 air", ""]));
    cell(&mut out, -8, "ladder [\"facing_direction\"=4]", true);
    cell(&mut out, 0, "vine [\"vine_direction_bits\"=8]", true);
    cell(&mut out, 8, "scaffolding", true);
    out.extend(owned(["tp @s -4.5 -60 -20.5", ""]));
    wait(&mut out, 3);
    for z in [-8, 0, 8] {
        for x in ["2.3", "2.5", "2.7"] {
            out.push(format!("summon zombie {x} -60 {z}.45 90 0"));
        }
    }
    wait(&mut out, 15);
    out.extend(owned(["kill @e[type=!player]", "fill -6 -60 -12 6 -54 12 air", ""]));
    cell(&mut out, 0, "ladder [\"facing_direction\"=4]", false);
    out.extend(owned(["tp @s 0.5 -60 0.5 -90 0", "summon zombie 2.5 -60 0.5 90 0", ""]));
    for _ in 0..4 {
        out.extend(owned(["!hit zombie", "", ""]));
    }
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=!player]", "fill -6 -60 -12 6 -54 12 air", ""]));
    out
}

const SHOWN: &[&str] = &["CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "EntityEvent", "LevelSoundEvent", "BlockEvent", "UpdateBlock", "LevelEvent"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ai2kin" => (kin(), SHOWN),
        "ai2push" => (push(), SHOWN),
        _ => return None,
    })
}
