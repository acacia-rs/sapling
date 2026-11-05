//! The thirteenth wave's group AG drills (research/parity/THIRTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27). Each geyser is a glass-walled column: magma or lava at -26, potent sulfur at -25 (set
//! `dormant`), a water source at -24 set after it, open air over it.
//! `ag2geyser`: one over magma and one over lava, a sculk sensor by each, both watched 40 s (the eruption, its
//! sound and its vibrations), then the water taken off the magma one. `ag2boost`: a pig in the water over a
//! geyser, its moves through two eruptions. `ag2far`: a geyser at x 1000, past where x * 3129871 wraps an int,
//! watched for its first eruption. `ag2reset`: a dry potent sulfur set `wet` and `dormant` where it stands.
//! `ag2lava`: a continuous geyser's lava changed by `/setblock` and `/fill`, a sensor listening.

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateSubchunkBlocks", "LevelEvent", "LevelEventGeneric", "LevelSoundEvent", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntity",
    "SetEntityMotion", "CommandOutput",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    let lines = match name {
        "ag2geyser" => geysers(),
        "ag2boost" => boost(),
        "ag2far" => far(),
        "ag2reset" => reset(),
        "ag2lava" => lava(),
        _ => return None,
    };
    Some((lines, SHOWN))
}

/// A geyser's column at `x z` over `under`, its potent sulfur set dormant with nothing over it yet.
fn column(x: i32, z: i32, under: &str) -> Vec<String> {
    vec![
        format!("fill {} -27 {} {} -27 {} stone", x - 1, z - 1, x + 1, z + 1),
        format!("fill {} -26 {} {} -22 {} glass", x - 1, z - 1, x + 1, z + 1),
        format!("setblock {x} -26 {z} {under}"),
        format!("fill {x} -24 {z} {x} -22 {z} air"),
        format!("setblock {x} -25 {z} potent_sulfur [\"potent_sulfur_state\"=\"dormant\"]"),
    ]
}

fn water(x: i32, z: i32) -> String {
    format!("setblock {x} -24 {z} water")
}

fn state(x: i32, z: i32, name: &str) -> String {
    format!("testforblock {x} -25 {z} potent_sulfur [\"potent_sulfur_state\"=\"{name}\"]")
}

fn cleared(lines: Vec<String>, x: i32, z: i32) -> Vec<String> {
    let clear = format!("fill {} -26 {} {} -22 {} air", x - 1, z - 1, x + 1, z + 1);
    with(lines, &[&clear, ""])
}

/// Over magma at 2 2 and over lava at -6 -6, a sensor four cells from each and from nothing else.
fn geysers() -> Vec<String> {
    let mut lines = floor();
    lines.extend(column(2, 2, "magma"));
    lines.extend(column(-6, -6, "lava"));
    lines.extend(["setblock 2 -26 6 sculk_sensor", "setblock -6 -26 -2 sculk_sensor", ""].map(String::from));
    lines.extend([state(2, 2, "dry"), water(2, 2), state(2, 2, "dormant"), water(-6, -6), state(-6, -6, "continuous")]);
    waits(&mut lines, 40);
    lines.extend(["setblock 2 -24 2 air".to_string(), state(2, 2, "dry"), String::new()]);
    waits(&mut lines, 4);
    lines = cleared(lines, 2, 2);
    lines = cleared(lines, -6, -6);
    with(lines, &["setblock 2 -26 6 air", "setblock -6 -26 -2 air", ""])
}

/// A pig in the water over the geyser at 2 2, through its first two eruptions.
fn boost() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule mobgriefing false"]);
    lines.extend(column(2, 2, "magma"));
    lines.extend([water(2, 2), "summon pig 2.5 -24 2.5".to_string(), String::new()]);
    waits(&mut lines, 70);
    lines = with(lines, &["kill @e[type=pig]"]);
    cleared(lines, 2, 2)
}

/// A geyser over magma at 1000 0, both players by it: its wait to the first eruption.
fn far() -> Vec<String> {
    // The far columns load once the players are there; they stand on a floor made after.
    let mut lines = with(floor(), &["tp @a 1003.5 -26 0.5", "fill 998 -27 -3 1006 -27 3 stone", "tp @a 1003.5 -26 0.5", ""]);
    lines.extend(column(1000, 0, "magma"));
    lines.push(water(1000, 0));
    waits(&mut lines, 36);
    lines = cleared(lines, 1000, 0);
    with(lines, &["tp @a 0.5 -26 0.5", "", "fill 998 -27 -3 1006 -27 3 air", ""])
}

/// A continuous geyser at -6 -6, a sensor by it: its lava set to stone, then lava again, then filled with
/// air alone, then lava again, then lava and sulfur filled together, then the column cleared.
fn lava() -> Vec<String> {
    let mut lines = floor();
    lines.extend(column(-6, -6, "lava"));
    lines.extend(["setblock -6 -26 -2 sculk_sensor".to_string(), String::new(), water(-6, -6), state(-6, -6, "continuous")]);
    waits(&mut lines, 3);
    lines.extend(["setblock -6 -26 -6 stone".to_string(), state(-6, -6, "wet"), state(-6, -6, "continuous")]);
    waits(&mut lines, 3);
    lines.extend(["setblock -6 -26 -6 lava".to_string(), state(-6, -6, "continuous")]);
    waits(&mut lines, 3);
    lines.extend(["fill -6 -26 -6 -6 -26 -6 air".to_string(), state(-6, -6, "wet")]);
    waits(&mut lines, 3);
    lines.extend(["setblock -6 -26 -6 lava".to_string(), state(-6, -6, "continuous")]);
    waits(&mut lines, 3);
    lines.extend(["fill -6 -26 -6 -6 -25 -6 air".to_string()]);
    waits(&mut lines, 3);
    lines = cleared(lines, -6, -6);
    waits(&mut lines, 3);
    with(lines, &["setblock -6 -26 -2 air", ""])
}

/// A potent sulfur on the floor, dry under air, then set wet and dormant in its place.
fn reset() -> Vec<String> {
    let mut lines = with(floor(), &["setblock 2 -26 2 potent_sulfur", ""]);
    lines.extend(["setblock 2 -26 2 potent_sulfur [\"potent_sulfur_state\"=\"wet\"]", "", "setblock 2 -26 2 potent_sulfur [\"potent_sulfur_state\"=\"dormant\"]", ""].map(String::from));
    lines.extend(["dry", "wet", "dormant"].map(|name| format!("testforblock 2 -26 2 potent_sulfur [\"potent_sulfur_state\"=\"{name}\"]")));
    with(lines, &["setblock 2 -26 2 air", ""])
}
