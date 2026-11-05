//! The twelfth wave's group 7 drills (research/parity/TWELFTH-BRIEF.md): the survival actor in lava,
//! stepped in and lava set round it, with and without resistance (`g7lava`).

use super::creatures::{floor, with};

const SHOWN: &[&str] = &["EntityEvent", "UpdateAttributes", "MobEffect", "UpdateBlock", "CommandOutput"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "g7lava" => (lava(), SHOWN),
        _ => return None,
    })
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// A lava cell walled in glass at x 3 and a water one at x -3: the actor is put in the lava, then
/// has lava set round it at x 6; two seconds each, put out in the water after. Unwarded, then
/// under resistance 255.
fn lava() -> Vec<String> {
    let mut lines = with(floor(), &["fill 2 -26 -1 4 -26 1 glass", "setblock 3 -26 0 lava", "fill -4 -26 -1 -2 -26 1 glass", "setblock -3 -26 0 water", "gamemode survival @s", ""]);
    for warded in [false, true] {
        let healed = ["effect @s health_boost 60 4 true", "effect @s instant_health 1 9 true", if warded { "effect @s resistance 60 255 true" } else { "" }];
        lines.extend(owned(healed));
        lines.extend(owned(["tp @s 3.5 -26 0.5", "", "", "tp @s -2.5 -26 0.5", "", ""]));
        lines.extend(owned(healed));
        lines.extend(owned(["tp @s 6.5 -26 0.5", "", "setblock 6 -26 0 lava", "", "", "fill 5 -26 -1 7 -26 1 air", "tp @s -2.5 -26 0.5", "", ""]));
        lines.extend(owned(["effect @s clear", "tp @s 0.5 -26 0.5", ""]));
    }
    with(lines, &["gamemode creative @s", "fill -8 -26 -8 16 -16 8 air", ""])
}
