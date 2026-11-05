//! thirteenth3s_drill.rs's scenes on a default world of seed 20261003, both players in creative.

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

/// Two snow golems on a stone floor at y 99 in the desert, left to melt.
pub fn melt() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(owned(["difficulty normal", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set noon", "gamemode creative @s", "gamemode creative CmdWatcher"]));
    go(&mut lines, [-6114, 100, -3508]);
    lines.extend(owned(["fill -6122 99 -3516 -6106 99 -3500 stone", "summon snow_golem -6110.5 100 -3507.5", "summon snow_golem -6117.5 100 -3511.5"]));
    waits(&mut lines, 12);
    lines.extend(owned(["testfor @e[type=snow_golem]", "kill @e[type=!player]"]));
    lines
}

/// Natural mobs come by the swamp hut for 75 s at midnight; then spawning stops, the players stand
/// 44 off, and the mobs past 32 and within 32 are counted each 5 s for 100 s.
pub fn despawn() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(owned(["difficulty normal", "gamerule domobspawning true", "gamerule dodaylightcycle false", "time set 18000", "gamemode creative @s", "gamemode creative CmdWatcher"]));
    go(&mut lines, [3843, 91, -1795]);
    waits(&mut lines, 67);
    lines.extend(owned(["gamerule domobspawning false", "testfor @e[type=!player]"]));
    go(&mut lines, [3843, 91, -1751]);
    for _ in 0..20 {
        lines.extend(owned(["testfor @e[type=!player,rm=32]", "testfor @e[type=!player,r=32]", "", "", ""]));
    }
    lines.push("kill @e[type=!player]".into());
    lines
}
