//! The thirteenth wave's group AO drills (research/parity/THIRTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27). `ao2rails`: rails by `/fill` and `/setblock` (one joined to by a rail that keeps its own
//! shape), then a 2x2x2 box moved by `/clone .. move`. `ao2boat`: boats summoned on the floor at fractions
//! of a block, a little above and into it, and in the air. `ao2push`: mobs summoned with their boxes into
//! single roof and wall blocks.

use super::creatures::{floor, waits, with};

const RAIL_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "CommandOutput"];
const SPAWN_SHOWN: &[&str] = &["AddEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ao2rails" => (rails(), RAIL_SHOWN),
        "ao2boat" => (boats(), SPAWN_SHOWN),
        "ao2push" => (pushes(), SPAWN_SHOWN),
        _ => return None,
    })
}

const BOTS_AWAY: [&str; 3] = ["tp CmdWatcher -6.5 -26 -6.5", "tp @s -6.5 -26 -5.5", ""];
const CLEAN: [&str; 5] = ["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""];

fn rails() -> Vec<String> {
    let mut lines = with(floor(), &BOTS_AWAY);
    lines.extend(["fill 2 -26 -2 6 -26 -2 rail", ""].map(String::from));
    // An east-west pair, then a rail north of its east end: it stays north-south, the end turns.
    lines.extend(["setblock 2 -26 -5 rail", "setblock 3 -26 -5 rail", "setblock 3 -26 -6 rail", ""].map(String::from));
    // The same by `/fill`: (9, 3) joins (9, 2) and the pair's end at (9, 4) without turning itself.
    lines.extend(["setblock 8 -26 4 rail", "setblock 9 -26 4 rail", "fill 9 -26 2 9 -26 3 rail", ""].map(String::from));
    let wools = ["red", "orange", "yellow", "lime", "blue", "purple", "white", "black"];
    let cells = [(2, -26, 6), (2, -26, 7), (2, -25, 6), (2, -25, 7), (3, -26, 6), (3, -26, 7), (3, -25, 6), (3, -25, 7)];
    lines.extend(cells.iter().zip(wools).map(|((x, y, z), wool)| format!("setblock {x} {y} {z} {wool}_wool")));
    lines.extend(["", "clone 2 -26 6 3 -25 7 6 -26 6 replace move", ""].map(String::from));
    waits(&mut lines, 1);
    with(lines, &CLEAN)
}

/// Boats on the floor at x fractions (z 0.85, which no edge pushes) and z fractions (x 0.85), at
/// half blocks, at y -25.8 and -26.2, and one in the air; then a lone boat at (4.5, 4.5) twice.
fn boats() -> Vec<String> {
    let mut lines = with(floor(), &BOTS_AWAY);
    let fractions = [0.35, 0.4, 0.45, 0.55, 0.6, 0.65, 0.5];
    for (i, f) in fractions.iter().enumerate() {
        let x = -4.0 + 3.0 * i as f64;
        lines.push(format!("summon boat {:.2} -26 -6.15", x + f));
        lines.push(format!("summon boat {:.2} -26 {:.2}", x + 0.85, -3.0 + f));
        lines.push(format!("summon boat {:.2} -26 1.5", x + 0.5));
    }
    for (x, y, z) in [(-3.5, -25.8, 4.5), (-0.5, -26.2, 4.5), (2.25, -26.2, 4.15), (5.5, -26.5, 4.5), (8.5, -25.9, 4.5), (11.5, -20.0, 4.5), (14.25, -26.0, 4.25)] {
        lines.push(format!("summon boat {x} {y} {z}"));
    }
    waits(&mut lines, 2);
    for _ in 0..2 {
        lines.extend(["kill @e[type=boat]", "", "summon boat 4.5 -26 4.5", ""].map(String::from));
        waits(&mut lines, 1);
    }
    with(lines, &CLEAN)
}

/// Single roof blocks a block up and single wall blocks: zombies, a cow, a horse and a llama under
/// roofs (one zombie off centre, one with a block south of its own cell), a pig into a block's corner.
fn pushes() -> Vec<String> {
    let mut lines = with(floor(), &BOTS_AWAY);
    let roofs = [(2, -6), (5, -6), (8, -6), (11, -6), (14, -6), (2, -2), (5, -2)];
    lines.extend(roofs.iter().map(|(x, z)| format!("setblock {x} -25 {z} stone")));
    lines.extend(["setblock 5 -26 -5 stone", "setblock 12 -26 -1 stone", ""].map(String::from));
    lines.extend(["summon zombie 2.5 -26 -5.5", "summon zombie 5.5 -26 -5.5", "summon zombie 8.3 -26 -5.5", "summon cow 11.5 -26 -5.5", "summon horse 14.5 -26 -5.5", "summon llama 2.5 -26 -1.5", "summon zombie 5.5 -26 -1.7", "summon pig 11.8 -26 -1.2"].map(String::from));
    waits(&mut lines, 2);
    with(lines, &CLEAN)
}
