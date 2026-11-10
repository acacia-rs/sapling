//! The fourteenth wave's group AH drills (research/parity/FOURTEENTH-BRIEF.md), at midnight on easy.
//! `ah1swim`: one squid in a pool 5 across and 5 deep (the ab1squid pool), and one cod in a pool 7
//! across and 4 deep beside it, for a minute.

use super::creatures::waits;

const SHOWN: &[&str] = &["CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "SetEntityMotion", "EntityEvent"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ah1swim" => (swim(), SHOWN),
        _ => return None,
    })
}

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

fn start() -> Vec<String> {
    owned(&["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "gamerule mobgriefing false", "weather clear", "difficulty easy", "time set 18000", "gamemode creative @a", "effect @a clear", "clear @s"])
}

fn swim() -> Vec<String> {
    let mut out = start();
    out.extend(owned(&["fill 18 -61 18 38 -54 28 air", "fill 20 -61 20 26 -55 26 stone", "fill 21 -60 21 25 -56 25 water", "fill 28 -61 20 36 -55 28 stone", "fill 29 -60 21 35 -57 27 water"]));
    out.extend(owned(&["tp CmdWatcher 17.5 -60 17.5", "tp @s 23.5 -54 17.5 0 0", ""]));
    out.extend(owned(&["summon squid 21.5 -58 21.5", "summon cod 32.5 -58 24.5"]));
    waits(&mut out, 60);
    out.extend(owned(&["kill @e[type=!player]", "fill 18 -61 18 38 -54 28 air", ""]));
    out
}
