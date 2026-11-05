//! thirteenth3w_drill.rs's scenes on a default world of seed 20261003, both players in creative.

use crate::creatures::waits;

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

/// Natural mobs come by the swamp hut for 75 s at midnight; then spawning stops and the players stand
/// 80 east, 5 chunks or more from the columns of the mobs west of the hut, and every mob is counted
/// each 5 s for 60 s; then the players go back.
pub fn despawn() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(owned(["difficulty normal", "gamerule domobspawning true", "gamerule dodaylightcycle false", "time set 18000", "gamemode creative @s", "gamemode creative CmdWatcher"]));
    go(&mut lines, [3843, 91, -1795]);
    waits(&mut lines, 67);
    lines.extend(owned(["gamerule domobspawning false", "testfor @e[type=!player]", "testfor @e[type=!player,x=3843,y=91,z=-1795,r=44]"]));
    go(&mut lines, [3923, 91, -1795]);
    for _ in 0..12 {
        lines.extend(owned(["testfor @e[type=!player]", "testfor @e[type=!player,r=44]", "", "", ""]));
    }
    go(&mut lines, [3843, 91, -1795]);
    lines.extend(owned(["testfor @e[type=!player]", "", "", "testfor @e[type=!player]", "kill @e[type=!player]"]));
    lines
}
