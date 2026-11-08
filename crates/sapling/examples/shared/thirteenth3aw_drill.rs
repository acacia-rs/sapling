//! The thirteenth wave's group AW drills (research/parity/THIRTEENTH-BRIEF.md). On a default world of seed
//! 20261003 (sapling `SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`, BDS with server-side
//! generation): `aw3despawn`, natural mobs by the swamp hut for 67 s at midnight, then spawning off and the
//! players 13 higher, so the mobs on the ground stand 32 to 44 off and stay ticked; every mob counted each
//! 5 s for 150 s, which pins the idle 30 s and 1 in 800 a tick roll.

use crate::creatures::waits;

const HUT: [i32; 3] = [3843, 91, -1795];
const ABOVE: i32 = 13;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// Both players to `at` on a barrier floor under it, laid again each second while its chunks load.
fn go(lines: &mut Vec<String>, [x, y, z]: [i32; 3]) {
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("tp {bot} {x} {y} {z}"));
    }
    for _ in 0..8 {
        lines.push(format!("fill {} {} {} {} {} {} barrier", x - 2, y - 1, z - 2, x + 2, y - 1, z + 2));
    }
}

fn despawn() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(owned(["difficulty normal", "gamerule domobspawning true", "gamerule dodaylightcycle false", "time set 18000", "gamemode creative @s", "gamemode creative CmdWatcher"]));
    go(&mut lines, HUT);
    waits(&mut lines, 67);
    lines.extend(owned(["gamerule domobspawning false", "testfor @e[type=!player]"]));
    let [x, y, z] = HUT;
    go(&mut lines, [x, y + ABOVE, z]);
    for _ in 0..30 {
        lines.extend(owned(["testfor @e[type=!player]", "testfor @e[type=!player,r=32]", "testfor @e[type=!player,rm=32,r=44]", "", ""]));
    }
    lines.extend(owned(["kill @e[type=!player]"]));
    lines
}

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "aw3despawn" => (despawn(), &["CommandOutput", "AddEntity", "RemoveEntity"]),
        _ => return None,
    })
}
