//! The thirteenth wave's group AZ drill (research/parity/THIRTEENTH-BRIEF.md), on a default world of seed
//! 20261003 (`SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`). `az3bastion`: both bots
//! straight to the bastion at (-328, 248) in the Nether, as `t12actors2`'s last stop, for a minute
//! (docs/research/structure-spawns.md "Template actors").

use crate::creatures::waits;

const SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MobEquipment", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "az3bastion" => (bastion(), SHOWN),
        _ => return None,
    })
}

fn bastion() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(["difficulty normal", "gamerule domobspawning true", "gamemode creative @s", "gamemode creative CmdWatcher"].map(String::from));
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("execute in nether run tp {bot} -328.5 100 248.5"));
    }
    for _ in 0..15 {
        lines.push("execute in nether run fill -329 99 247 -327 99 249 barrier".into());
    }
    waits(&mut lines, 60);
    lines
}
