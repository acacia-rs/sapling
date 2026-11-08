//! The fourteenth wave's group AB drills (research/parity/FOURTEENTH-BRIEF.md), at midnight on easy.
//! `ab1land`: a cow dropped down a glass shaft from 0 to 3 blocks, then into water, a sculk sensor
//! read by a comparator into dust beside it. `ab1wither`: a wither summoned 12 off an iron golem held
//! in a barrier pen (resistance, healed), the players creative, its head yaw logged against a target
//! that stays put. `ab1squid`: three squids in a 5-wide pool. `ab1flock`: six dolphins in a 13-wide pool.
//! `ab1wither2`: the wither among three cows held still in the open. `ab1patrol`: a night with the
//! level's time past 6000 s (`time set` rather than level.dat's `currentTick`), patrol events on.

use super::creatures::waits;

const SHOWN: &[&str] = &[
    "CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "SetEntityMotion", "EntityEvent", "LevelSoundEvent", "LevelEvent",
    "LevelEventGeneric", "UpdateBlock",
];
const PATROL_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ab1land" => (land(), SHOWN),
        "ab1wither" => (wither(), SHOWN),
        "ab1squid" => (squid(), SHOWN),
        "ab1flock" => (flock(), SHOWN),
        "ab1wither2" => (wither_cows(), SHOWN),
        "ab1patrol" => (patrol(), PATROL_SHOWN),
        _ => return None,
    })
}

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

fn start() -> Vec<String> {
    owned(&["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "gamerule mobgriefing false", "weather clear", "difficulty easy", "time set 18000", "gamemode creative @a", "effect @a clear", "clear @s"])
}

/// A cow summoned in a 1-wide glass shaft at heights 0, 0.3, 1, 2 and 3 over the ground, then over
/// water 2 deep; each part killed after and the sensor left to settle.
fn land() -> Vec<String> {
    let mut out = start();
    out.extend(owned(&["fill -12 -60 -12 12 -50 16 air", "tp CmdWatcher 0.5 -60 -10.5", "tp @s -6.5 -60 6.5 0 0", ""]));
    out.extend(owned(&["setblock 4 -60 6 sculk_sensor", "setblock 5 -60 6 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\"]", "setblock 6 -60 6 redstone_wire"]));
    out.extend(owned(&["fill -1 -60 1 1 -55 3 glass", "fill 0 -60 2 0 -55 2 air", "fill -1 -62 -3 1 -55 -1 glass", "fill 0 -62 -2 0 -55 -2 air", "fill 0 -62 -2 0 -61 -2 water", ""]));
    waits(&mut out, 4);
    for (y, z) in [("-60", "2.5"), ("-59.7", "2.5"), ("-59", "2.5"), ("-58", "2.5"), ("-57", "2.5"), ("-58", "-1.5")] {
        out.push(format!("summon cow 0.5 {y} {z}"));
        waits(&mut out, 4);
        out.push("kill @e[type=cow]".into());
        waits(&mut out, 4);
    }
    out.extend(owned(&["kill @e[type=!player]", "fill -1 -62 -3 6 -55 7 air", "fill -1 -62 -3 1 -61 -1 stone", ""]));
    out
}

/// The wither and its one target, an iron golem that cannot move or die, for a minute; then the golem
/// goes and the wither is watched with no target.
fn wither() -> Vec<String> {
    let mut out = start();
    out.extend(owned(&["fill -20 -60 -20 20 -40 20 air", "tp CmdWatcher -10.5 -60 10.5", "tp @s -12.5 -60 -6.5 0 0", ""]));
    out.extend(owned(&["fill 7 -60 7 10 -57 10 barrier", "fill 8 -60 8 9 -57 9 air", "summon iron_golem 9 -60 9", "effect @e[type=iron_golem] slowness 1000 255 true", "effect @e[type=iron_golem] resistance 1000 255 true", "summon wither 0.5 -60 -3.5"]));
    for _ in 0..70 {
        out.push("effect @e[type=iron_golem] instant_health 1 4 true".into());
    }
    out.extend(owned(&["kill @e[type=iron_golem]"]));
    waits(&mut out, 20);
    out.extend(owned(&["kill @e[type=!player]", "fill 7 -60 7 10 -57 10 air", ""]));
    out
}

/// The wither among three cows that cannot move or die, out in the open, for 70 s; then the cows go.
fn wither_cows() -> Vec<String> {
    let mut out = start();
    out.extend(owned(&["fill -20 -60 -20 20 -40 20 air", "tp CmdWatcher -14.5 -60 14.5", "tp @s -16.5 -60 -12.5 0 0", ""]));
    for at in ["10.5 -60 -3.5", "-7.5 -60 4.5", "0.5 -60 9.5"] {
        out.push(format!("summon cow {at}"));
    }
    out.extend(owned(&["effect @e[type=cow] slowness 1000 255 true", "effect @e[type=cow] resistance 1000 255 true", "summon wither 0.5 -60 -3.5"]));
    for _ in 0..70 {
        out.push("effect @e[type=cow] instant_health 1 4 true".into());
    }
    out.push("kill @e[type=cow]".into());
    waits(&mut out, 20);
    out.extend(owned(&["kill @e[type=!player]", ""]));
    out
}

/// A night on a level whose time says 5.75 days, mob spawning and patrol events on, for 240 s.
fn patrol() -> Vec<String> {
    let mut out = owned(&["kill @e[type=!player]", "gamerule domobspawning true", "gamerule dodaylightcycle false", "weather clear", "difficulty easy", "time set 138000"]);
    out.extend(owned(&["time query gametime", "time query daytime", "gamemode creative @a", "tp CmdWatcher 0.5 -60 0.5", "tp @s 0.5 -60 0.5"]));
    out.extend(owned(&["mobevent minecraft:pillager_patrols_event true", "mobevent events_enabled true", ""]));
    waits(&mut out, 240);
    out.extend(owned(&["time query gametime", "gamerule domobspawning false", "kill @e[type=!player]", ""]));
    out
}

/// Three squids in a pool 5 across and 5 deep for a minute.
fn squid() -> Vec<String> {
    let mut out = start();
    out.extend(owned(&["fill 18 -61 18 28 -54 28 air", "fill 20 -61 20 26 -55 26 stone", "fill 21 -60 21 25 -56 25 water", "tp CmdWatcher 17.5 -60 17.5", "tp @s 23.5 -54 17.5 0 0", ""]));
    out.extend(owned(&["summon squid 21.5 -58 21.5", "summon squid 24.5 -58 24.5", "summon squid 23.5 -57 22.5"]));
    waits(&mut out, 60);
    out.extend(owned(&["kill @e[type=!player]", "fill 20 -61 20 26 -55 26 air", ""]));
    out
}

/// Six dolphins spread over a pool 13 across and 7 deep, open to the air, for 90 s.
fn flock() -> Vec<String> {
    let mut out = start();
    out.extend(owned(&["fill -8 -27 -17 8 -14 17 air", "fill -7 -27 -7 7 -20 7 stone", "fill -6 -26 -6 6 -20 6 water", "tp CmdWatcher 7.5 -19 0.5", "tp @s -7.5 -19 0.5 0 0", ""]));
    for at in ["-5.5 -24 -5.5", "5.5 -24 -5.5", "-5.5 -24 5.5", "5.5 -24 5.5", "0.5 -22 0.5", "0.5 -25 -4.5"] {
        out.push(format!("summon dolphin {at}"));
    }
    waits(&mut out, 90);
    out.extend(owned(&["kill @e[type=!player]", "fill -8 -27 -17 8 -14 17 air", ""]));
    out
}
