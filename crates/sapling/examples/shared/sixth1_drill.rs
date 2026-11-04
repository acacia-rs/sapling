//! The sixth wave's head drill (research/parity/SIXTH-BRIEF.md "Group 1"): slowed cows watching the
//! bots and slowed cows left alone, near enough that every tick's move reaches both bots (`glance`).

use super::creatures::{floor, waits, with};

const GLANCE_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "UpdateAttributes"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "glance" => (glance(), GLANCE_SHOWN),
        _ => return None,
    })
}

/// Three slowed cows 3 blocks from the bots (within `look_at_player`'s 6) and three 9 off (outside
/// it, but under the 10.7 at which a cow's moves start to be paced), watched 60 s.
fn glance() -> Vec<String> {
    let mut lines = with(floor(), &["tp @s 52.5 -60 0.5", "tp CmdWatcher 52.5 -60 2.5", ""]);
    for at in ["55.5 -60 -1.5", "55.5 -60 1.5", "55.5 -60 4.5", "61.5 -60 -1.5", "61.5 -60 1.5", "61.5 -60 4.5"] {
        lines.push(format!("summon cow {at}"));
    }
    lines.push("effect @e[type=cow] slowness 1000 255 true".into());
    waits(&mut lines, 60);
    with(lines, &["kill @e[type=cow]", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}
