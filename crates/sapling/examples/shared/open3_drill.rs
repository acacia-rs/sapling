//! Drills of the still-open wave's effects (docs/research splash-sound.md, mob-damage.md): what a
//! cow killed in a pool leaves and where, an enderman in rain on open ground. On
//! creature_drill.rs's floor (stone at y -27, the bots at y -26).

use super::creatures::{floor, waits, with};

/// Packets shown of these drills, moves too.
const SHOWN: &[&str] = &[
    "UpdateBlock", "LevelEvent", "LevelSoundEvent", "AddItemEntity", "AddEntity", "RemoveEntity", "EntityEvent", "UpdateAttributes", "MoveEntityAbsolute", "MoveEntityDelta",
    "SetEntityMotion", "SetEntityData",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "wetloot" => (wet_loot(), SHOWN),
        "enderrain" => (ender_rain(), SHOWN),
        _ => return None,
    })
}

/// wetflags' pool 3 deep at x 3-5, z 3-5: a cow dropped in, killed 4 s later, its loot followed.
fn wet_loot() -> Vec<String> {
    let mut lines = with(floor(), &["fill 2 -27 2 6 -24 6 stone", "fill 3 -26 3 5 -24 5 water", "", "summon cow 4.5 -21 4.5"]);
    waits(&mut lines, 4);
    lines.push("kill @e[type=cow]".to_owned());
    waits(&mut lines, 4);
    lines.extend(with(Vec::new(), &["kill @e[type=item]", "fill 2 -27 2 6 -24 6 air", "fill 2 -27 2 6 -27 6 stone"]));
    lines
}

/// An enderman on the flat ground at y -60, 60 east of the floor (its teleports stay clear of the
/// floor's shade and inside sapling's ground), the bots beside it, rain for 8 s.
fn ender_rain() -> Vec<String> {
    let mut lines = with(floor(), &["tp @s 52.5 -60 0.5", "tp CmdWatcher 52.5 -60 2.5", "", "summon enderman 60.5 -60 0.5", "", "weather rain"]);
    waits(&mut lines, 8);
    lines.extend(with(Vec::new(), &["weather clear", "kill @e[type=enderman]", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""]));
    lines
}
