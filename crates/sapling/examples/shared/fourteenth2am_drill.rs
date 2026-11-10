//! Fourteenth wave, group AM (drill2): `am2ride`, six spear riders on zombie horses charging the
//! survival actor from the north one after another, so passes of both servers can be paired by the
//! horses' rolled `movement`. `am2push`, ai2push with every zombie summoned grown (ai2push's ladder
//! row had a baby jockey) and the ladder row's clear of the ladder's plate and the walls.

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

fn riders() -> Vec<String> {
    let mut out = field();
    out.extend(owned(["fill -14 -60 -14 14 -54 14 air", "fill -14 -61 -14 14 -61 14 grass", ""]));
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 3 true", "effect @s health_boost 1000 9 true"]));
    for _ in 0..6 {
        out.extend(owned(["kill @e[type=!player]", "effect @s instant_health 1 9 true", "tp @s 0.5 -60 0.5 180 0", ""]));
        out.push("summon zombie_horse 0.5 -60 -8.5 0 0 minecraft:spawn_adult_with_rider".into());
        wait(&mut out, 8);
    }
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
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

const LADDER: &str = "ladder [\"facing_direction\"=4]";

fn push() -> Vec<String> {
    let mut out = field();
    out.extend(owned(["fill -6 -61 -12 6 -61 12 stone", "fill -6 -60 -12 6 -54 12 air", ""]));
    cell(&mut out, -8, LADDER, true);
    cell(&mut out, 0, "vine [\"vine_direction_bits\"=8]", true);
    cell(&mut out, 8, "scaffolding", true);
    out.extend(owned(["tp @s -4.5 -60 -20.5", ""]));
    wait(&mut out, 3);
    // The ladder's plate is x 2.8125..3: boxes 0.6 wide clear it and the west wall from 2.3 to 2.5.
    // ai2push's ladder row at z -8.45 lay in the wall cell z -9.
    for (z, xs) in [("-7.55", ["2.3", "2.4", "2.5"]), ("0.45", ["2.3", "2.5", "2.7"]), ("8.45", ["2.3", "2.5", "2.7"])] {
        for x in xs {
            out.push(format!("summon zombie {x} -60 {z} 90 0 minecraft:as_adult"));
        }
    }
    wait(&mut out, 15);
    out.extend(owned(["kill @e[type=!player]", "fill -6 -60 -12 6 -54 12 air", ""]));
    cell(&mut out, 0, LADDER, false);
    out.extend(owned(["tp @s 0.5 -60 0.5 -90 0", "summon zombie 2.5 -60 0.5 90 0 minecraft:as_adult", ""]));
    for _ in 0..4 {
        out.extend(owned(["!hit zombie", "", ""]));
    }
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=!player]", "fill -6 -60 -12 6 -54 12 air", ""]));
    out
}

const SHOWN: &[&str] = &["CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "SetEntityMotion", "EntityEvent", "LevelSoundEvent", "UpdateBlock"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "am2ride" => (riders(), SHOWN),
        "am2push" => (push(), SHOWN),
        _ => return None,
    })
}
