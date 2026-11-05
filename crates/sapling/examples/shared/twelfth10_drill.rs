//! The twelfth wave's group 10 drills (research/parity/TWELFTH-BRIEF.md): the warded actor in lava,
//! three rounds of stepping in and of lava set round it (`g10lava`); arrows at a filled row of
//! glass panes and at a lone pane, off their posts (`g10pane`); an arrow left in a block set to air,
//! then a flaming one at TNT (`g10flame`); a breeze under jump boost leaping at the survival actor
//! (`g10breeze`).

use super::creatures::{floor, waits, with};

const LAVA_SHOWN: &[&str] = &["EntityEvent", "UpdateAttributes", "MobEffect", "UpdateBlock", "CommandOutput"];
const SHOT_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "SetEntityMotion", "MoveEntityAbsolute", "MoveEntityDelta", "EntityEvent", "LevelSoundEvent", "LevelEvent", "UpdateBlock", "CommandOutput"];
const LEAP_SHOWN: &[&str] = &["AddEntity", "SetEntityMotion", "MoveEntityAbsolute", "MoveEntityDelta", "MobEffect", "CommandOutput"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "g10lava" => (lava(), LAVA_SHOWN),
        "g10pane" => (panes(), SHOT_SHOWN),
        "g10flame" => (unstuck(), SHOT_SHOWN),
        "g10breeze" => (breeze(), LEAP_SHOWN),
        _ => return None,
    })
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// g7lava's warded rounds three times: put in the lava at x 3, then lava set round it at x 6.
fn lava() -> Vec<String> {
    let mut lines = with(floor(), &["fill 2 -26 -1 4 -26 1 glass", "setblock 3 -26 0 lava", "fill -4 -26 -1 -2 -26 1 glass", "setblock -3 -26 0 water", "gamemode survival @s", "effect @s resistance 120 255 true", ""]);
    for _ in 0..3 {
        lines.extend(owned(["tp @s 3.5 -26 0.5", "", "", "tp @s -2.5 -26 0.5", "", ""]));
        lines.extend(owned(["tp @s 6.5 -26 0.5", "", "setblock 6 -26 0 lava", "", "", "fill 5 -26 -1 7 -26 1 air", "tp @s -2.5 -26 0.5", "", ""]));
    }
    with(lines, &["effect @s clear", "tp @s 0.5 -26 0.5", "gamemode creative @s", "fill -8 -26 -8 16 -16 8 air", ""])
}

/// The actor's bow shot from (0.5, -26, 0.5) at the point `look` off its feet, watched 3 s.
fn shot(lines: &mut Vec<String>, look: &str) {
    lines.extend(owned(["tp @s 0.5 -26 0.5 -90 0", ""]));
    lines.extend([format!("!look {look}"), "!use".into(), String::new(), "!release".into()]);
    waits(lines, 3);
    lines.push("kill @e[type=arrow]".into());
}

/// A row of panes along z at x 8 filled in, shot 0.3 off a post; a lone pane at z 6 shot the same; a
/// stone wall at x 12 behind both.
fn panes() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "tp CmdWatcher 0.5 -26 -6.5", "replaceitem entity @s slot.weapon.mainhand 0 bow", "fill 12 -26 -4 12 -22 8 stone", "fill 8 -26 -2 8 -24 2 glass_pane", "setblock 8 -26 6 glass_pane", "setblock 8 -25 6 glass_pane", ""]);
    shot(&mut lines, "8 1.4 -0.3");
    shot(&mut lines, "8 1.4 5.7");
    shot(&mut lines, "8 1.4 0");
    with(lines, &["clear @s", "fill -8 -26 -8 16 -16 8 air", ""])
}

/// An arrow stuck in stone on a post, the stone then set to air; then a flaming arrow at TNT there.
fn unstuck() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "tp CmdWatcher 0.5 -26 5.5", "replaceitem entity @s slot.weapon.mainhand 0 bow", "setblock 8 -26 0 stone", "setblock 8 -25 0 stone", ""]);
    lines.extend(owned(["tp @s 0.5 -26 0.5 -90 0", "", "!look 8 1.4 0", "!use", "", "!release", "", "setblock 8 -25 0 air", "", "", "kill @e[type=arrow]"]));
    lines.extend(owned(["setblock 8 -25 0 tnt", "enchant @s flame 1", ""]));
    lines.extend(owned(["tp @s 0.5 -26 0.5 -90 0", "", "!look 8 1.4 0", "!use", "", "!release", "", ""]));
    waits(&mut lines, 5);
    with(lines, &["kill @e[type=!player]", "clear @s", "fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -16 8 air", ""])
}

/// A breeze 9 off the survival actor (warded) under jump boost V, watched 25 s.
fn breeze() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 -7.5", "gamemode survival @s", "effect @s resistance 60 255 true", "summon breeze 9.5 -26 0.5", "effect @e[type=breeze] jump_boost 60 4 true"]);
    waits(&mut lines, 25);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -16 8 air", ""])
}
