//! Fourteenth wave, group AP (drill2). `ap2kill`: held zombies `/kill`ed beside a sculk sensor read by a
//! comparator into dust (the dust's runtime id gives the frequency heard), in the open, in am2push's walled
//! vine, scaffolding and ladder cells and on a 3×3 stone top six up, loot off, then the open and the top
//! with loot. `ap2flee`: cows hit by the actor from the west and from the north, each left to flee.
//! `ap2ride`: am2ride's spear riders with the actor's own moves shown.

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
        "tp CmdWatcher 0.5 -60 -30.5",
        "fill -14 -61 -14 14 -61 14 stone",
        "fill -14 -60 -14 14 -54 14 air",
        "",
    ])
}

/// Where the zombie of a part stands, and what is built round it at x 1..3 of row z 0.
const PARTS: &[(&str, &[&str])] = &[
    ("2.5 -60 0.5", &[]),
    ("2.5 -60 0.45", &["fill 1 -60 -1 3 -55 1 stone", "fill 2 -60 0 2 -55 0 vine [\"vine_direction_bits\"=8]"]),
    ("2.5 -60 0.45", &["fill 1 -60 -1 3 -55 1 stone", "fill 2 -60 0 2 -55 0 scaffolding"]),
    ("2.4 -60 0.45", &["fill 1 -60 -1 3 -55 1 stone", "fill 2 -60 0 2 -55 0 ladder [\"facing_direction\"=4]"]),
    ("2.5 -54 0.5", &["fill 1 -60 -1 3 -55 1 stone"]),
];

/// One part: the sensor at x -2 of row 0 with its comparator and dust east of it, the zombie held
/// still so it is not heard stepping, a `/kill` once the sensor has settled, and a clean field after.
fn part(out: &mut Vec<String>, at: &str, built: &[&str]) {
    out.extend(built.iter().map(|line| line.to_string()));
    out.extend(owned(["setblock -2 -60 0 sculk_sensor", "setblock -1 -60 0 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\"]", "setblock 0 -60 0 redstone_wire"]));
    out.push(format!("summon zombie {at} 90 0 minecraft:as_adult"));
    out.push("effect @e[type=zombie] slowness 100 255 true".into());
    wait(out, 3);
    out.push("kill @e[type=zombie]".into());
    wait(out, 4);
    out.extend(owned(["kill @e[type=!player]", "fill -6 -60 -4 6 -54 4 air", ""]));
}

fn kills() -> Vec<String> {
    let mut out = field();
    out.extend(owned(["tp @s -4.5 -60 -12.5", "gamerule domobloot false", ""]));
    for (at, built) in PARTS {
        part(&mut out, at, built);
    }
    out.push("gamerule domobloot true".into());
    for (at, built) in [PARTS[0], PARTS[4]] {
        part(&mut out, at, built);
    }
    out
}

/// Three cows hit from the west, then three from the north, each fleeing for five seconds.
fn flights() -> Vec<String> {
    let mut out = field();
    for (actor, cow) in [("0.5 -60 0.5 -90 0", "2.5 -60 0.5"); 3].into_iter().chain([("0.5 -60 0.5 0 0", "0.5 -60 2.5"); 3]) {
        out.extend(owned(["kill @e[type=!player]", ""]));
        out.push(format!("tp @s {actor}"));
        out.push(format!("summon cow {cow} 0 0 minecraft:entity_spawned"));
        out.extend(owned(["", "!hit cow"]));
        wait(&mut out, 5);
    }
    out.extend(owned(["kill @e[type=!player]", ""]));
    out
}

/// am2ride's six spear riders, with the watcher 16 east of the actor so the actor's own moves are
/// shown (`MovePlayer`).
fn riders() -> Vec<String> {
    let mut out = field();
    out.extend(owned(["tp CmdWatcher 16.5 -60 0.5", "fill -14 -61 -14 14 -61 14 grass", ""]));
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 3 true", "effect @s health_boost 1000 9 true"]));
    for _ in 0..6 {
        out.extend(owned(["kill @e[type=!player]", "effect @s instant_health 1 9 true", "tp @s 0.5 -60 0.5 180 0", ""]));
        out.push("summon zombie_horse 0.5 -60 -8.5 0 0 minecraft:spawn_adult_with_rider".into());
        wait(&mut out, 8);
    }
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

const RIDE_SHOWN: &[&str] = &["CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "MovePlayer", "SetEntityData", "SetEntityMotion", "EntityEvent"];

const SHOWN: &[&str] = &["CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "SetEntityMotion", "EntityEvent", "LevelSoundEvent", "UpdateBlock"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ap2kill" => (kills(), SHOWN),
        "ap2flee" => (flights(), SHOWN),
        "ap2ride" => (riders(), RIDE_SHOWN),
        _ => return None,
    })
}
