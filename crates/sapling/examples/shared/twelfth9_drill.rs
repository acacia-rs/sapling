//! The twelfth wave's group 9 drills (research/parity/TWELFTH-BRIEF.md), on a default world of seed
//! 20261003 as twelfth3_drill.rs's (`SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`):
//! `t9spawns` stands 30 to 40 off the outpost of chunk (117, -71) at noon, 31 off the swamp hut of
//! chunk (240, -114) at midnight and over the Nether fortress of chunks (11..19, 0..6), 150 s each
//! (docs/research/structure-spawns.md "The pass, drilled again"). `t9patrol`, on the flat world
//! (`SAPLING_DRILL_WORLD=open`), summons a patrol captain and two followers and watches them 20 s
//! (structure-spawns.md "Patrols").

use crate::creatures::waits;

const SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "Text", "CommandOutput"];
const MOVES: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityAbsolute", "MoveEntityDelta", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "t9spawns" => (spawns(), SHOWN),
        "t9patrol" => (patrol([60, 0]), MOVES),
        "t9patrol2" => (patrol([40, 30]), MOVES),
        "t9patrol3" => (patrol([-60, 0]), MOVES),
        _ => return None,
    })
}

/// The captain at `[x, z]` on the ground, the followers 6 off it on x and on z.
fn patrol([x, z]: [i32; 2]) -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(["difficulty normal", "gamerule domobspawning false", "gamemode creative @s", "gamemode creative CmdWatcher", "time set 6000"].map(String::from));
    for (dx, dz, event) in [(0, 0, "promote_to_patrol_captain"), (6, 0, "spawn_as_patrol_follower"), (0, 6, "spawn_as_patrol_follower")] {
        lines.push(format!("summon pillager {}.5 -60 {}.5 0 0 minecraft:{event}", x + dx, z + dz));
    }
    waits(&mut lines, 20);
    lines.push("kill @e[type=pillager]".into());
    lines
}

/// Both bots on a barrier pad at `[x, y, z]`: the fill is asked once a second until the column
/// has loaded (sapling generates it first), then both stand on it.
fn stand(lines: &mut Vec<String>, [x, y, z]: [i32; 3]) {
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("tp {bot} {x} {} {z}", y + 1));
    }
    for _ in 0..15 {
        lines.push(format!("fill {} {y} {} {} {y} {} barrier", x - 2, z - 2, x + 2, z + 2));
    }
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("tp {bot} {} {} {}", x as f32 + 0.5, y + 1, z as f32 + 0.5));
    }
}

fn spawns() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(["difficulty normal", "gamerule domobspawning true", "gamerule dodaylightcycle false", "time set 6000"].map(String::from));
    stand(&mut lines, [1880, 150, -1095]);
    waits(&mut lines, 150);
    lines.extend(["testfor @e[type=pillager]".to_string(), "time set 18000".into()]);
    stand(&mut lines, [3843, 90, -1795]);
    waits(&mut lines, 150);
    lines.push("testfor @e[type=witch]".into());
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("execute in nether run tp {bot} 248 100 60"));
    }
    stand(&mut lines, [248, 100, 60]);
    waits(&mut lines, 150);
    lines.extend(["blaze", "wither_skeleton", "skeleton", "magma_cube", "zombie_pigman"].map(|kind| format!("testfor @e[type={kind}]")));
    lines
}
