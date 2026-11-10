//! The fourteenth wave's group AN drills (research/parity/FOURTEENTH-BRIEF.md), on a default world of
//! seed 20261003 (BDS: a copy whose server.properties says so; sapling: drill_server with
//! `SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`). `an1gen`: the world made and the
//! nearest village found. `an1patrol`: both players creative at the spawn at midnight, patrol events
//! on, for 300 s; the AddEntity lines show where and when each pillager came (docs/research/patrols.md).

use super::creatures::waits;

const PATROL_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "an1gen" => (made(), PATROL_SHOWN),
        "an1patrol" => (patrol(), PATROL_SHOWN),
        _ => return None,
    })
}

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

fn made() -> Vec<String> {
    let mut out = owned(&["gamerule domobspawning false", "gamemode creative @a", "time query gametime"]);
    waits(&mut out, 5);
    out.extend(owned(&["locate structure village", "tp @s ~ ~ ~", ""]));
    out
}

fn patrol() -> Vec<String> {
    let mut out = owned(&["kill @e[type=!player]", "gamerule domobspawning true", "gamerule dodaylightcycle false", "weather clear", "difficulty easy", "time set 18000"]);
    out.extend(owned(&["time query gametime", "gamemode creative @a", "tp @s ~ ~ ~", "locate structure village"]));
    out.extend(owned(&["mobevent minecraft:pillager_patrols_event true", "mobevent events_enabled true", ""]));
    waits(&mut out, 300);
    out.extend(owned(&["time query gametime", "testfor @e[type=pillager]", "gamerule domobspawning false", "kill @e[type=!player]", ""]));
    out
}
