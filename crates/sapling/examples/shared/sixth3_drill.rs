//! The sixth wave's entity data drill (research/parity/SIXTH-BRIEF.md "Group 3"): the SetEntityData
//! that follows each new entity's announcement (mobs, TNT by command and by signal, an arrow, a
//! snowball, a dropped block, an orb), and what a horse-family mob's data does when it is hurt and
//! when it dies (`firstdata`).

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &["AddEntity", "AddItemEntity", "SetEntityData", "MoveEntityDelta", "RemoveEntity"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "firstdata" => (first_data(), SHOWN),
        _ => return None,
    })
}

/// Horse kinds and a llama (no horse flags) with their spawn events; the zombie horse by night,
/// as it burns by day.
const MOUNTS: &[(&str, &str)] = &[("horse", "spawn_tame_adult"), ("donkey", "spawn_tame_adult"), ("skeleton_horse", "spawn_adult"), ("llama", "spawn_tame_adult"), ("zombie_horse", "spawn_tame_adult")];

fn first_data() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "gamerule domobloot true", "summon cow 4.5 -26 0.5", "", "summon zombie 4.5 -26 3.5", "", "summon arrow 2.5 -22 -3.5", ""]);
    lines.extend(["summon tnt 8.5 -26 -3.5", "", "", "", "", "setblock 10 -26 3 tnt", "setblock 10 -26 4 redstone_block", "", "", "", "", ""].map(String::from));
    lines.extend(["setblock 6 -26 -6 oak_planks", "setblock 6 -26 -6 air destroy", "", "summon xp_orb 6.5 -25 6.5", "", "summon snowball 2.5 -20 6.5", ""].map(String::from));
    lines.extend(["kill @e[type=cow]", "", "", "kill @e[type=zombie]", "", "", "kill @e[type=item]", "kill @e[type=xp_orb]", ""].map(String::from));
    for (kind, event) in MOUNTS {
        if *kind == "zombie_horse" {
            lines.push("time set midnight".into());
        }
        lines.extend([format!("summon {kind} 2.5 -26 -6.5 0 0 minecraft:{event}"), format!("effect @e[type={kind}] slowness 200 255 true"), String::new(), String::new()]);
        lines.extend([format!("damage @e[type={kind}] 2"), String::new(), String::new(), String::new()]);
        lines.extend([format!("kill @e[type={kind}]"), String::new(), String::new(), "kill @e[type=item]".into(), String::new()]);
    }
    waits(&mut lines, 1);
    with(lines, &["time set noon"])
}
