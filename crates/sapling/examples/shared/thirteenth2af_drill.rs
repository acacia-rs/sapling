//! The thirteenth wave's group AF drills (research/parity/THIRTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27, the actor at 0.5 -26 0.5 facing east; clicks are offsets from its cell). `af2spring`: both
//! players at seed 20261003's sulfur spring (chunk -751,-752, potent sulfur at -12004 67 -12032 with three ticks
//! waiting; BDS runs it on the canon ac2 world, `KEEP_WORLD=1`). `af2sulfur`: potent sulfur set under water,
//! under air and dormant. `af2vault`: a survival key click on a vault, the same player's second key, a stick.
//! `af2load`: a spring template loaded with its ticks, its potent sulfur watched (with `BDS_GDB`, its waits).

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "BlockEvent", "LevelEvent", "LevelEventGeneric", "LevelSoundEvent", "AddEntity", "AddItemEntity", "RemoveEntity",
    "SpawnParticleEffect", "SetEntityMotion", "EntityEvent", "CommandOutput",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    let lines = match name {
        "af2spring" => spring(),
        "af2sulfur" => sulfur(),
        "af2vault" => vault(),
        "af2load" => load(),
        _ => return None,
    };
    Some((lines, SHOWN))
}

/// Both players over the spring for 40 s: its potent sulfur's ticks fall due 85, 309 and 369 ticks on.
fn spring() -> Vec<String> {
    let mut lines: Vec<String> = ["gamemode creative @a", "tp @a -12001 72 -12029"].map(String::from).into();
    lines.extend(states());
    waits(&mut lines, 40);
    lines.extend(states());
    lines
}

/// Which state the spring's ticked potent sulfur is in, and what is over it.
fn states() -> Vec<String> {
    let state = |name: &str| format!("testforblock -12004 67 -12032 potent_sulfur [\"potent_sulfur_state\"=\"{name}\"]");
    let mut lines: Vec<String> = ["dormant", "wet", "erupting"].map(state).into();
    lines.extend(["testforblock -12004 68 -12032 ice", "testforblock -12004 68 -12032 water"].map(String::from));
    lines
}

/// Potent sulfur set under a water source, under air, dormant under water, then each watched 40 s.
fn sulfur() -> Vec<String> {
    let mut lines = with(floor(), &[
        "setblock 2 -27 2 potent_sulfur",
        "setblock 2 -26 2 water",
        "setblock 4 -27 2 potent_sulfur",
        "setblock 6 -27 2 potent_sulfur [\"potent_sulfur_state\"=\"dormant\"]",
        "setblock 6 -26 2 water",
        "",
    ]);
    lines.extend(sulfur_states());
    waits(&mut lines, 40);
    lines.extend(sulfur_states());
    with(lines, &["fill 2 -27 2 6 -27 2 stone", "fill 2 -26 2 6 -26 2 air", ""])
}

/// sulfur_spring_medium_2 loaded on the floor: its dormant potent sulfur at 6 -21 13 under water, with the template's
/// ticks (85, 309, 369) if a load queues them; its state each 2 s for 24 s.
fn load() -> Vec<String> {
    let mut lines = with(floor(), &["structure load \"minecraft:sulfur_spring_medium_2\" 0 -26 8", ""]);
    let state = |name: &str| format!("testforblock 6 -21 13 potent_sulfur [\"potent_sulfur_state\"=\"{name}\"]");
    for _ in 0..12 {
        lines.extend(["dormant", "erupting"].map(state));
    }
    with(lines, &["testforblock 6 -20 13 water", "fill -2 -26 6 14 -15 22 air", ""])
}

/// The three set cells' potent sulfur states.
fn sulfur_states() -> Vec<String> {
    let test = |x: i32, name: &str| format!("testforblock {x} -27 2 potent_sulfur [\"potent_sulfur_state\"=\"{name}\"]");
    [2, 4, 6].into_iter().flat_map(|x| ["dormant", "wet", "erupting"].map(|name| test(x, name))).collect()
}

/// A survival actor opens a vault with a trial key; clicks a second vault with a stick (the wrong item); once
/// the first is done ejecting, clicks it again with its second key (rewarded already).
fn vault() -> Vec<String> {
    let setup = ["setblock 2 -26 -2 vault", "setblock 2 -26 2 vault", "gamemode survival @s", "replaceitem entity @s slot.weapon.mainhand 0 trial_key 2", "", ""];
    let mut lines = with(floor(), &setup);
    lines = with(lines, &["!click 2 0 -2 west", ""]);
    lines = with(lines, &["replaceitem entity @s slot.weapon.mainhand 0 stick 1", "", "!click 2 0 2 up", ""]);
    waits(&mut lines, 6);
    lines = with(lines, &["replaceitem entity @s slot.weapon.mainhand 0 trial_key 1", "", "!click 2 0 -2 west", ""]);
    waits(&mut lines, 2);
    with(lines, &["gamemode creative @s", "kill @e[type=item]", "fill 2 -26 -2 2 -26 2 air", ""])
}
