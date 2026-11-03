//! Drills of the still-open wave's effects (docs/research splash-sound.md, mob-damage.md): what a
//! cow killed in a pool leaves and where, an enderman in rain on open ground, mobs past the
//! bots' `tick-distance`. On
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
        "farmobs" => (far_mobs(), SHOWN),
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

/// Mobs held by slowness, moved from beside the bots (chunk 3, 0) to chunks (6, -2), (6, -3),
/// (7, 0) and (8, 0): which leave sight past `tick-distance` 4 (bds-enderrain.txt's enderman did at
/// (6, -3)), then each moved back, which a mob the level unloaded does not answer.
fn far_mobs() -> Vec<String> {
    let far = [("cow", "100.5 -60 -24.5"), ("pig", "100.5 -60 -40.5"), ("sheep", "116.5 -60 0.5"), ("chicken", "132.5 -60 0.5")];
    let mut lines = with(floor(), &["tp @s 52.5 -60 0.5", "tp CmdWatcher 52.5 -60 2.5", ""]);
    for (kind, _) in far {
        lines.extend([format!("summon {kind} 56.5 -60 0.5"), format!("effect @e[type={kind}] slowness 60 255 true")]);
    }
    for (kind, at) in far {
        lines.push(format!("tp @e[type={kind}] {at}"));
        waits(&mut lines, 2);
    }
    for (kind, _) in far {
        lines.push(format!("tp @e[type={kind}] 56.5 -60 0.5"));
    }
    waits(&mut lines, 2);
    lines.extend(with(Vec::new(), &["kill @e[type=!player]", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""]));
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
