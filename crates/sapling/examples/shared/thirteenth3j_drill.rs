//! The thirteenth wave's group J drills (research/parity/THIRTEENTH-BRIEF.md), on a default world of seed
//! 20261003 (`SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`). `jxmansion`: both bots by the
//! mansion at (-8184, 15944) a minute, as `t12actors2`'s last stop (its five extra mobs in chunk x -511).

use crate::creatures::waits;

const SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "jxmansion" => (mansion(), SHOWN),
        "jxoutpost" => (outpost(), SHOWN),
        _ => return None,
    })
}

/// Both bots over `[x, y, z]` for `secs` seconds, in creative, on a barrier pad (twelfth12_drill.rs's `visit`).
fn visit(lines: &mut Vec<String>, [x, y, z]: [i32; 3], secs: usize) {
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("tp {bot} {x}.5 {y} {z}.5"));
    }
    for _ in 0..15 {
        lines.push(format!("fill {} {} {} {} {} {} barrier", x - 1, y - 1, z - 1, x + 1, y - 1, z + 1));
    }
    waits(lines, secs);
}

fn mansion() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(["difficulty normal", "gamerule domobspawning true", "gamemode creative @s", "gamemode creative CmdWatcher", "time set 6000"].map(String::from));
    visit(&mut lines, [-8184, 140, 15944], 60);
    lines.extend(["testfor @e[type=evocation_illager]", "testfor @e[type=vindicator]"].map(String::from));
    lines
}

/// By the outpost of chunk (117, -71) 40 s, so the columns around it generate and BDS saves their spawn records.
fn outpost() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(["gamerule domobspawning false", "gamemode creative @s", "gamemode creative CmdWatcher"].map(String::from));
    visit(&mut lines, [1880, 160, -1125], 40);
    lines
}
