//! The thirteenth wave's group AL drills (research/parity/THIRTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27). `al2fill`: three water cells held in a glass ring copied out by `/clone`, by
//! `/structure load` and by a `/clone .. move`, each left 3 s (which of them flow); then a floating line of
//! sand filled, its cells shown with the falling blocks that took them.

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "AddEntity", "RemoveEntity", "LevelSoundEvent", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    let lines = match name {
        "al2fill" => fills(),
        _ => return None,
    };
    Some((lines, SHOWN))
}

/// The glass ring holding three water cells at x 2 to 4, z -7.
const RING: [&str; 2] = ["fill 1 -26 -8 5 -26 -6 glass", "fill 2 -26 -7 4 -26 -7 water"];
/// Takes every water off the floor (the 3 s of flow reach its edges).
const CLEAR: &str = "fill -8 -26 -8 16 -24 8 air";

fn fills() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "tp @s -6.5 -26 -5.5", ""]);
    lines.extend(RING.map(String::from));
    lines.extend(["", "structure save al2w 2 -26 -7 4 -26 -7", "", "clone 2 -26 -7 4 -26 -7 2 -26 -3", ""].map(String::from));
    waits(&mut lines, 3);
    lines.extend([CLEAR, "", "structure load al2w 8 -26 -3", ""].map(String::from));
    waits(&mut lines, 3);
    lines.extend([CLEAR, ""].map(String::from));
    lines.extend(RING.map(String::from));
    lines.extend(["", "clone 2 -26 -7 4 -26 -7 2 -26 -3 replace move", ""].map(String::from));
    waits(&mut lines, 3);
    lines.extend([CLEAR, "", "fill 2 -22 4 4 -22 4 sand", ""].map(String::from));
    waits(&mut lines, 2);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 stone", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}
