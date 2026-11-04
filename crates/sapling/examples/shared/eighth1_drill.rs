//! The eighth wave's mob body drills (research/parity/EIGHTH-BRIEF.md "Group 1", mob-damage.md
//! "Where sapling differs"): mobs dropped through air, water and lava, mobs left inside blocks set
//! where they stand, cows shut in water, and the loot of a `/kill`. All on harm_drill.rs's floor,
//! within 10.7 of the actor so every tick's move is told.

use super::harm::{SHOWN, mid, pen, start, summon, wait};

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "bodies" => bodies(),
            "lifts" => lifts(),
            _ => return None,
        },
        SHOWN,
    ))
}

fn bodies() -> Vec<String> {
    let mut lines = start("noon", 0);
    // Falls: a chicken and a cow 10 up in the open; a chicken and a pig put on a lava cell, a chicken on two of water.
    lines.extend([summon("chicken", 4.5, -16.0, -3.5), summon("cow", 7.5, -16.0, -3.5)]);
    for (x, top, liquid) in [(-4, -26, "lava"), (-7, -26, "lava"), (-10, -25, "water")] {
        pen(&mut lines, x, -4, 3);
        lines.push(format!("fill {x} -26 -4 {x} {top} -4 {liquid}"));
    }
    lines.extend([summon("chicken", mid(-4), -25.0, -3.5), summon("pig", mid(-7), -25.0, -3.5), summon("chicken", mid(-10), -24.0, -3.5)]);
    wait(&mut lines, 5);
    lines.extend(["kill @e[type=!player]".into(), "fill -11 -26 -5 8 -16 -3 air".into(), String::new(), String::new()]);
    // Blocks set where mobs stand: one cell, two cells, a bottom slab, and a pig in one cell.
    let set = [(-4, "cow", "setblock -4 -26 4 stone"), (-7, "cow", "fill -7 -26 4 -7 -25 4 stone"), (4, "cow", "setblock 4 -26 4 oak_slab"), (7, "pig", "setblock 7 -26 4 stone")];
    for (x, kind, _) in set {
        pen(&mut lines, x, 4, 4);
        lines.push(summon(kind, mid(x), -26.0, 4.5));
    }
    wait(&mut lines, 2);
    lines.extend(set.map(|(_, _, setting)| setting.to_string()));
    wait(&mut lines, 4);
    // Cows in water two deep: shut under a stone roof, and open to the sky.
    lines.extend(["kill @e[type=!player]".into(), "fill -8 -26 3 8 -16 5 air".into(), String::new()]);
    for x in [-3, 3] {
        pen(&mut lines, x, 8, 4);
        lines.push(format!("fill {x} -26 8 {x} -25 8 water"));
    }
    lines.extend(["setblock -3 -24 8 stone".into(), summon("cow", mid(-3), -26.0, 8.5), summon("cow", mid(3), -26.0, 8.5)]);
    wait(&mut lines, 8);
    // Loot: two cows and a chicken killed at once.
    lines.extend([summon("cow", 3.5, -26.0, -2.5), summon("cow", -2.5, -26.0, -2.5), summon("chicken", 0.5, -26.0, -3.5), String::new()]);
    lines.extend(["kill @e[type=!player]".into(), String::new(), String::new(), "fill -11 -26 -5 8 -16 9 air".into(), String::new()]);
    lines
}

/// How high a block set in a standing cow's cell lifts it onto its top: eight blocks from 0.375 to
/// a full one; then an iron golem and a cow in lava a second, the lava made stone (mob-damage.md).
fn lifts() -> Vec<String> {
    let mut lines = start("noon", 0);
    let set = ["daylight_detector", "oak_slab", "stonecutter_block", "enchanting_table", "end_portal_frame", "soul_sand", "farmland", "stone"];
    let cell = |at: usize| (-7 + 2 * at as i32, if at % 2 == 0 { 4 } else { 7 });
    for at in 0..set.len() {
        let (x, z) = cell(at);
        pen(&mut lines, x, z, 4);
        lines.push(summon("cow", mid(x), -26.0, mid(z)));
    }
    wait(&mut lines, 2);
    lines.extend(set.iter().enumerate().map(|(at, block)| {
        let (x, z) = cell(at);
        format!("setblock {x} -26 {z} {block}")
    }));
    wait(&mut lines, 3);
    lines.extend(["kill @e[type=!player]".into(), "fill -9 -26 3 9 -16 8 air".into(), String::new()]);
    lines.extend(["fill -6 -26 -8 -2 -24 -4 stone", "fill -5 -26 -7 -3 -24 -5 air", "fill -5 -26 -7 -3 -26 -5 lava", "summon iron_golem -3.5 -25 -5.5"].map(String::from));
    pen(&mut lines, 3, -6, 3);
    lines.extend(["setblock 3 -26 -6 lava".into(), summon("cow", 3.5, -25.0, -5.5), String::new()]);
    lines.extend(["fill -5 -26 -7 -3 -26 -5 stone", "setblock 3 -26 -6 stone"].map(String::from));
    wait(&mut lines, 4);
    lines.extend(["kill @e[type=!player]".into(), String::new(), "fill -7 -26 -9 5 -16 -3 air".into(), String::new()]);
    lines
}
