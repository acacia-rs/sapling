//! Drills of what reads the rain's level as it sets in and clears (docs/research/mob-damage.md,
//! "What the rain's level drives"): daylight detectors at noon, fire on open stone and a riptide
//! trident in the seconds after `/weather clear`, while the rain still falls. On the open flat
//! ground at y -60, east of creature_drill.rs's floor (`SAPLING_DRILL_WORLD=open` on sapling).

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &["UpdateBlock", "UpdateSubChunkBlocks", "LevelEvent", "LevelSoundEvent", "SetEntityData", "AddEntity", "CorrectPlayerMovePrediction"];
/// 49 cells of fire over stone; `fill … air` answers how many still burn.
const FIRES: &str = "56 -60 -6 62 -60 0";

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "rainramp" => (rain_ramp(), SHOWN),
        _ => return None,
    })
}

/// Noon held; a detector and an inverted one; fire for 4 s dry; 6 s of rain, then clear: fire laid
/// one second after and put out 4 s later, a riptide trident used and let go in between; a dry
/// riptide; 8 s of thunder, clear.
fn rain_ramp() -> Vec<String> {
    let (lay, douse) = (format!("fill {FIRES} fire"), format!("fill {FIRES} air"));
    let mut lines = with(floor(), &["gamerule dodaylightcycle false", "time set 6000", "gamerule dofiretick true", "tp @s 52.5 -60 0.5 90 0", "tp CmdWatcher 52.5 -60 2.5"]);
    lines.extend(with(Vec::new(), &["fill 56 -61 -6 62 -61 0 stone", &douse, "setblock 50 -60 4 daylight_detector", "setblock 51 -60 4 daylight_detector_inverted"]));
    lines.extend(with(Vec::new(), &["replaceitem entity @s slot.weapon.mainhand 0 trident", "enchant @s riptide 1", &lay]));
    waits(&mut lines, 4);
    lines.push(douse.clone());
    lines.push("weather rain".into());
    waits(&mut lines, 6);
    lines.extend(with(Vec::new(), &["weather clear", &lay, "!use", "!release", "", "", &douse]));
    lines.extend(with(Vec::new(), &["tp @s 52.5 -60 0.5 90 0", "", "!use", "", "!release", ""]));
    lines.push("weather thunder".into());
    waits(&mut lines, 8);
    lines.push("weather clear".into());
    waits(&mut lines, 7);
    lines.extend(with(Vec::new(), &["fill 50 -60 4 51 -60 4 air", "gamerule dodaylightcycle true", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""]));
    lines
}
