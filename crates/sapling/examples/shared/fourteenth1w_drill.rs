//! The fourteenth wave's group W drills (research/parity/FOURTEENTH-BRIEF.md). `w1pops`, on the flat
//! ground at midnight: a fresh warden held 10 off and a sculk sensor read by a comparator into dust,
//! while a door loses its lower half, a torch its block, and dropped items are `/kill`ed.

use super::creatures::waits;

const POPS_SHOWN: &[&str] = &["AddEntity", "AddItemEntity", "RemoveEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "LevelEventGeneric", "CommandOutput", "UpdateBlock"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "w1pops" => (pops(), POPS_SHOWN),
        _ => return None,
    })
}

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

const FRESH: [&str; 3] = ["kill @e[type=warden]", "summon warden 0.5 -60 10.5", "effect @e[type=warden] slowness 1000 255 true"];

/// Each part after a fresh held warden: a door's lower half set to air, a torch's block set to air,
/// one dropped item `/kill`ed, three dropped items `/kill`ed. The dust east of the comparator shows
/// the frequency the sensor last heard.
fn pops() -> Vec<String> {
    let mut out = owned(&["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "weather clear", "difficulty easy", "time set 18000"]);
    out.extend(owned(&["gamemode creative @a", "effect @a clear", "clear @s", "fill -12 -60 -12 12 -50 16 air", "tp CmdWatcher 0.5 -60 -14.5", "tp @s 0.5 -60 0.5 0 0", ""]));
    out.extend(owned(&["setblock 4 -60 6 sculk_sensor", "setblock 5 -60 6 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\"]", "setblock 6 -60 6 redstone_wire"]));
    out.extend(owned(&["setblock -3 -60 6 wooden_door", "setblock -3 -60 3 stone", "setblock -3 -59 3 torch", ""]));
    let parts: [&[&str]; 4] = [
        &["setblock -3 -60 6 air"],
        &["setblock -3 -60 3 air"],
        &["setblock 2 -60 2 stone", "setblock 2 -60 2 air destroy", "", "", "", "", "kill @e[type=item]"],
        &["setblock 2 -60 2 stone", "setblock 2 -60 2 air destroy", "setblock 1 -60 2 stone", "setblock 1 -60 2 air destroy", "setblock 3 -60 3 stone", "setblock 3 -60 3 air destroy", "", "", "", "", "kill @e[type=item]"],
    ];
    for part in parts {
        out.extend(owned(&FRESH));
        waits(&mut out, 4);
        out.extend(owned(part));
        waits(&mut out, 5);
    }
    out.extend(owned(&["kill @e[type=!player]", "fill -4 -60 2 6 -59 7 air", "tp @s 0.5 -60 0.5 0 0", ""]));
    out
}
