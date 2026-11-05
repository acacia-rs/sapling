//! The twelfth wave's structure spawn drill (research/parity/TWELFTH-BRIEF.md "Group 2"), on a default
//! world of seed 20261003 (BDS: a copy whose server.properties says so; sapling: drill_server with
//! `SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`): both bots on a barrier pad 40 blocks
//! off the swamp hut of chunk (240, -114) at noon for 150 s, then at midnight for 150 s, then off the
//! pillager outpost of chunk (117, -71) at noon for 150 s (`t3spawns`); nearer the hut, over the
//! monument and in the fortress (`t3deep`). The AddEntity lines show what came where
//! (docs/research/structure-spawns.md).

use crate::creatures::waits;

const SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "Text", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "t3spawns" => (spawns(), SHOWN),
        "t3deep" => (deep(), SHOWN),
        _ => return None,
    })
}

/// Both bots on a pad at `[x, y, z]`, the chunks about it loaded first.
fn stand(lines: &mut Vec<String>, [x, y, z]: [i32; 3]) {
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("tp {bot} {x} {} {z}", y + 1));
    }
    waits(lines, 10);
    lines.push(format!("fill {} {y} {} {} {y} {} barrier", x - 2, z - 2, x + 2, z + 2));
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("tp {bot} {x}.5 {} {z}.5", y + 1));
    }
}

/// Normal difficulty at noon; the first seconds wait, as sapling's generated world dropped the first lines.
fn setup() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(["difficulty normal", "gamerule domobspawning true", "gamerule dodaylightcycle false", "time set 6000"].map(String::from));
    lines
}

fn spawns() -> Vec<String> {
    let mut lines = setup();
    stand(&mut lines, [3843, 100, -1785]);
    waits(&mut lines, 150);
    lines.extend(["time set 18000".to_string(), "testfor @e[type=witch]".into()]);
    waits(&mut lines, 150);
    lines.extend(["testfor @e[type=witch]".to_string(), "time set 6000".into()]);
    stand(&mut lines, [1880, 170, -1095]);
    waits(&mut lines, 150);
    lines.push("testfor @e[type=pillager]".into());
    lines
}

/// At noon: 31 blocks off the hut's middle for 150 s (inside the 44 a natural mob despawns past),
/// above the monument of chunks (-25..-21, -24..-20) for 150 s, then in the Nether over the
/// fortress of chunks (11..19, 0..6) for 150 s.
fn deep() -> Vec<String> {
    let mut lines = setup();
    stand(&mut lines, [3843, 90, -1795]);
    waits(&mut lines, 150);
    stand(&mut lines, [-360, 75, -345]);
    waits(&mut lines, 150);
    lines.push("testfor @e[type=guardian]".into());
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("execute in nether run tp {bot} 248 100 60"));
    }
    stand(&mut lines, [248, 100, 60]);
    waits(&mut lines, 150);
    lines.extend(["testfor @e[type=blaze]", "testfor @e[type=wither_skeleton]", "testfor @e[type=magma_cube]"].map(String::from));
    lines
}
