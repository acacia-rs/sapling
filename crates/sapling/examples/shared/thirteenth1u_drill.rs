//! The thirteenth wave's group U drills (research/parity/THIRTEENTH-BRIEF.md): leads and knots
//! (`uuknot`), boats (`uuboat`), where a summoned mob is put in a tight space (`uuspawn`) and sculk
//! sensors by amethyst (`uusculk`, `uureson`).

use crate::creatures::{floor, waits, with};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "uuknot" => (knots(), KNOT_SHOWN),
        "uuboat" => (boats(), BOAT_SHOWN),
        "uuspawn" => (spawns(), SPAWN_SHOWN),
        "uusculk" => (sculk(), SCULK_SHOWN),
        "uureson" => (resonance(), SCULK_SHOWN),
        _ => return None,
    })
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

const KNOT_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "AddItemEntity", "SetEntityLink", "SetEntityData", "MoveEntityDelta", "MoveEntityAbsolute", "LevelSoundEvent", "CommandOutput"];

/// A cow led to an oak fence and tied there; the cow killed, leaving the knot holding nothing; a
/// second cow tied, then let go by a click on the knot. Then a held cow (slowness 255) whose holder
/// is put 7 to 15 blocks off, a fresh cow each time, to find where the lead snaps.
fn knots() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 -7.5", "setblock 4 -26 3 oak_fence", ""]);
    for second in [false, true] {
        lines.extend(["kill @e[type=cow]", "summon cow 2.5 -26 3.5", "effect @e[type=cow] slowness 999 255 true", &hold("lead 4"), "tp @s 1.5 -26 3.5 -90 0", "", "!interact cow", "", "!click 3 0 0 west", ""].map(String::from));
        if second {
            lines.extend(["!interact leash_knot", ""].map(String::from));
            waits(&mut lines, 3);
        } else {
            lines.extend(["kill @e[type=cow]", ""].map(String::from));
            waits(&mut lines, 8);
        }
    }
    lines.extend(["kill @e[type=!player]", "setblock 4 -26 3 air", ""].map(String::from));
    for off in 7..=15 {
        lines.extend(["kill @e[type=!player]", "summon cow -6.5 -26 -3.5", "effect @e[type=cow] slowness 999 255 true", &hold("lead 1"), "tp @s -7.5 -26 -3.5 90 0", "", "!interact cow", ""].map(String::from));
        lines.extend([format!("tp @s {} -26 -3.5", -6.5 + off as f32), String::new(), String::new()]);
    }
    with(lines, &["kill @e[type=!player]", "clear @s", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

const BOAT_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "AddItemEntity", "SetEntityLink", "SetEntityMotion", "MoveEntityDelta", "MoveEntityAbsolute", "EntityEvent", "LevelSoundEvent", "CorrectPlayerMovePrediction", "CommandOutput"];

/// Boats summoned at whole, half and odd spots and turned; boats dropped 20 and 6 blocks onto stone;
/// a pig riding a boat on a pool, then the pool raised two over it; the actor in a boat so drowned.
fn boats() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 -7.5", "tp @s 0.5 -26 -7.5", ""]);
    for (x, z, yaw) in [(4.5, 4.5, 0), (2.0, 2.0, 0), (6.3, 2.8, 0), (9.5, 4.5, 90), (12.5, 4.5, 45), (14.5, 0.5, 180)] {
        lines.push(format!("summon boat {x} -26 {z} {yaw} 0"));
    }
    waits(&mut lines, 3);
    lines.extend(["kill @e[type=boat]", "kill @e[type=item]", "summon boat 4.5 -6 -4.5", "summon boat 10.5 -20 -4.5"].map(String::from));
    waits(&mut lines, 5);
    lines.extend(["kill @e[type=boat]", "kill @e[type=item]", "fill -7 -31 -7 -1 -27 -1 stone", "fill -6 -30 -6 -2 -27 -2 water", "summon boat -3.5 -27 -3.5", "summon pig -3.5 -26 -3.5", "", "ride @e[type=pig] start_riding @e[type=boat,c=1]", ""].map(String::from));
    lines.extend(["fill -6 -26 -6 -2 -25 -2 water", ""].map(String::from));
    waits(&mut lines, 4);
    lines.extend(["kill @e[type=!player]", "fill -6 -26 -6 -2 -25 -2 air", "", "summon boat -3.5 -27 -3.5", "", "ride @s start_riding @e[type=boat,c=1]", "", "fill -6 -26 -6 -2 -25 -2 water", ""].map(String::from));
    waits(&mut lines, 4);
    with(lines, &["ride @s stop_riding", "kill @e[type=!player]", "fill -7 -31 -7 -1 -16 -1 air", "fill -7 -27 -7 -1 -27 -1 stone", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

const SPAWN_SHOWN: &[&str] = &["AddEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "CommandOutput"];

/// Mobs summoned with their boxes into stone: a cow and a zombie under ceilings 0.3 and 0.4 too low,
/// a zombie in a one-wide shaft under a ceiling, mobs against a wall to the east and north, a cow
/// with its top a block into a ceiling, and a pig and a cow in the open.
fn spawns() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 -7.5", "tp @s 0.5 -26 -7.5", ""]);
    lines.extend(["fill 2 -25 2 3 -25 3 stone", "fill 5 -25 2 6 -25 3 stone", "fill 8 -26 2 8 -26 3 stone", "fill 11 -25 1 11 -25 1 stone", "fill 10 -26 1 10 -26 1 stone", "fill 12 -26 1 12 -26 1 stone", "fill 11 -26 0 11 -26 0 stone", "fill 11 -26 2 11 -26 2 stone"].map(String::from));
    lines.extend(["fill 14 -26 -3 14 -24 3 stone", "fill 2 -26 -5 6 -24 -5 stone", "fill 2 -25 -2 3 -25 -1 stone", ""].map(String::from));
    // Cow (1.3) under a ceiling 1 up: 0.3 in; zombie (1.9) with feet at -26.5 under it: 0.4 in.
    lines.extend(["summon cow 2.5 -26 2.5", "summon zombie 5.5 -26.5 2.5", "summon zombie 7.9 -26 2.5", "summon zombie 11.5 -26 1.5", "summon pig 13.8 -26 0.5", "summon zombie 13.9 -26 -2.5", "summon cow 4.5 -26 -4.3", "summon pig 2.5 -26 -1.5", "summon cow 7.5 -26 -1.5"].map(String::from));
    waits(&mut lines, 3);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

const SCULK_SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "LevelSoundEvent", "LevelEvent", "LevelEventGeneric", "BlockEvent", "SpawnParticleEffect", "BlockActorData", "CommandOutput"];

/// A sculk sensor with an amethyst block beside it hears a plank set 3.6 off; a second sensor 7 from
/// the amethyst and 10.4 from the plank hears only what the amethyst passes on. Then a calibrated
/// sensor and a shrieker are set and left, for the world folder's block actors.
fn sculk() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "tp @s -6.5 -26 -5.5", ""]);
    lines.extend(["setblock 4 -26 0 sculk_sensor", "setblock 5 -26 0 amethyst_block", "setblock 12 -26 0 sculk_sensor", "setblock 12 -26 6 calibrated_sculk_sensor", "setblock 8 -26 6 sculk_shrieker"].map(String::from));
    waits(&mut lines, 3);
    for _ in 0..2 {
        lines.extend(["setblock 2 -26 3 oak_planks", ""].map(String::from));
        waits(&mut lines, 4);
        lines.extend(["setblock 2 -26 3 air", ""].map(String::from));
        waits(&mut lines, 4);
    }
    lines.extend(["setblock 5 -26 0 air", "", "setblock 2 -26 3 oak_planks", ""].map(String::from));
    waits(&mut lines, 4);
    with(lines, &["setblock 2 -26 3 air", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

/// As `uusculk`, but heard from the actor teleported 3.5 from the first sensor (BDS heard that,
/// a `/setblock` not): with the amethyst beside it, then without.
fn resonance() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "tp @s -6.5 -26 -5.5", ""]);
    lines.extend(["setblock 4 -26 0 sculk_sensor", "setblock 5 -26 0 amethyst_block", "setblock 12 -26 0 sculk_sensor"].map(String::from));
    waits(&mut lines, 3);
    for last in ["setblock 5 -26 0 air", "setblock 4 -26 0 air"] {
        lines.extend(["tp @s 0.5 -26 0.5", ""].map(String::from));
        waits(&mut lines, 4);
        lines.extend(["tp @s -6.5 -26 -5.5".to_owned(), String::new(), last.to_owned(), String::new()]);
        waits(&mut lines, 3);
    }
    with(lines, &["setblock 12 -26 0 air", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}
