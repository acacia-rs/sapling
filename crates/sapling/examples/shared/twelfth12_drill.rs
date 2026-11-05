//! The twelfth wave's group 12 drills (research/parity/TWELFTH-BRIEF.md), on a default world of seed
//! 20261003 as twelfth3_drill.rs's (`SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`):
//! `t12locate` asks where the structures whose templates carry actors are; `t12actors` stands by
//! each a while, so their chunks generate (BDS saves them on stop) and the actors show
//! (docs/research/structure-spawns.md "Template actors").

use crate::creatures::waits;

const SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "t12locate" => (locate(), SHOWN),
        "t12actors" => (actors(false), SHOWN),
        "t12actors2" => (actors(true), SHOWN),
        _ => return None,
    })
}

const OVERWORLD: &[&str] = &["village", "igloo", "mansion", "pillager_outpost", "abandoned_camp", "minecraft:abandoned_camp"];

fn locate() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    for at in ["0 100 0", "2000 100 -1000", "-2000 100 2000"] {
        lines.push(format!("tp @s {at}"));
        waits(&mut lines, 2);
        lines.extend(OVERWORLD.iter().map(|name| format!("locate structure {name}")));
    }
    lines.push("execute in nether run locate structure bastion_remnant".into());
    lines
}

/// Both bots over `[x, y, z]` for `secs` seconds, in creative so nothing hunts them.
fn visit(lines: &mut Vec<String>, [x, y, z]: [i32; 3], secs: usize) {
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("tp {bot} {x}.5 {y} {z}.5"));
    }
    waits(lines, secs);
}

/// The template actors' drill, with `doMobSpawning` `rule` (BDS placed none with it off).
fn actors(rule: bool) -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(["difficulty normal".to_string(), format!("gamerule domobspawning {rule}"), "gamemode creative @s".into(), "gamemode creative CmdWatcher".into(), "time set 6000".into()]);
    visit(&mut lines, [1880, 160, -1125], 20);
    visit(&mut lines, [1752, 140, -1016], 25);
    visit(&mut lines, [-984, 140, 993], 25);
    visit(&mut lines, [-8184, 140, 15944], 40);
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("execute in nether run tp {bot} -328 100 248"));
    }
    waits(&mut lines, 25);
    lines
}
