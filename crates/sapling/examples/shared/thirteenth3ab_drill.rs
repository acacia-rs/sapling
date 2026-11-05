//! The thirteenth wave's group AB drills (research/parity/THIRTEENTH-BRIEF.md), on the flat ground.
//! `ab3sight`: trial spawners each 8 off the survival actor behind a wall of glass panes, leaves,
//! iron bars, tinted glass or a stained glass pane (which see-through blocks hide a player).
//! `ab3golem`: `y3baby` with the village golem left free (whether its slowness kept it off the hitter).

use super::creatures::waits;
use super::thirteenth3y::{TRIAL_END, TRIAL_SHOWN, TRIAL_START, trial_site};
use crate::villages::{UNSKILLED, VILLAGE_SHOWN, village};

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ab3sight" => (sight(), TRIAL_SHOWN),
        "ab3golem" => (golem(), VILLAGE_SHOWN),
        _ => return None,
    })
}

/// A wall of `block` across the way from a trial spawner at (`x`, -60, 0) to the actor 8.5 south.
fn behind(lines: &mut Vec<String>, x: i32, block: &str) {
    let wall = format!("fill {} -60 4 {} -57 4 {block}", x - 1, x + 1);
    trial_site(lines, x, -60, &[&wall], &format!("{x}.5 -60 8.5"));
}

fn sight() -> Vec<String> {
    let mut lines: Vec<String> = TRIAL_START.map(String::from).into();
    for (x, block) in [(300, "glass_pane"), (340, "oak_leaves [\"persistent_bit\"=true]"), (380, "iron_bars"), (420, "tinted_glass"), (460, "red_stained_glass_pane")] {
        behind(&mut lines, x, block);
    }
    lines.extend(TRIAL_END.map(String::from));
    lines
}

fn golem() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(["summon iron_golem 10.5 -60 -6.5 0 0 minecraft:from_village", "summon villager 2.5 -60 3.5 0 0 minecraft:entity_born", "effect @e[type=villager_v2] slowness 1000 255 true"].map(String::from));
    waits(&mut out, 8);
    out.push("tag @e[type=villager_v2,x=2.5,y=-60,z=3.5,r=1] add hit".into());
    for _ in 0..10 {
        out.extend(["execute at @e[tag=hit] run tp @s ~ ~ ~-1.5 facing ~ ~0.5 ~", "!hit villager_v2", "effect @e[tag=hit] instant_health 1 4"].map(String::from));
    }
    waits(&mut out, 30);
    out
}
