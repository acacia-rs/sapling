//! The thirteenth wave's group AU drills (research/parity/THIRTEENTH-BRIEF.md). `au3charge`: ominous
//! trial spawners in twelve kept-table regions (30 by 20 by 30) under roofs, four round each of three
//! region corners, the actor among each four for 40 s: what an item spawner carries when its table
//! kept a stack of charges, and what it throws. Run the BDS side with `level-seed=20261018`, the drill
//! server's. `au3brush`: brush strokes held with the actor's own ticks running (`!au3brush`), so a
//! stroke's uses come a tick apart as a vanilla client's, with pauses between.

use std::time::Duration;

use super::creatures::{waits, with};
use super::thirteenth3y::{TRIAL_END, TRIAL_SHOWN, TRIAL_START, spawner};
use super::warden::start;

const BLOCKS: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "LevelSoundEvent", "LevelEvent", "AddItemEntity", "CommandOutput"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "au3charge" => (charge(), TRIAL_SHOWN),
        "au3brush" => (brush(), BLOCKS),
        _ => return None,
    })
}

/// Region corners on the drill server's flat ground (x up to 224, z -48 to 48), open sky.
const CORNERS: [[i32; 2]; 3] = [[120, 0], [180, 30], [180, -30]];

fn charge() -> Vec<String> {
    let mut lines: Vec<String> = TRIAL_START.map(String::from).into();
    for [x, z] in CORNERS {
        lines.push(format!("fill {} -50 {} {} -50 {} stone", x - 20, z - 20, x + 20, z + 20));
        for [dx, dz] in [[-4, -4], [4, -4], [-4, 4], [4, 4]] {
            spawner(&mut lines, [x + dx, -60, z + dz], true);
        }
    }
    for [x, z] in CORNERS {
        lines.extend([format!("tp @s {x}.5 -60 {z}.5"), "gamemode survival @s".to_owned()]);
        for _ in 0..20 {
            lines.push(format!("tp @s {x}.5 -60 {z}.5"));
            waits(&mut lines, 1);
        }
        lines.push("gamemode creative @s".to_owned());
    }
    with(lines, &TRIAL_END)
}

/// Gravel then sand, each brushed in held strokes with pauses shorter than the 40-tick fall-back.
fn brush() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode survival @s", "setblock 2 -28 0 stone", "setblock -2 -28 0 stone", "setblock 2 -27 0 suspicious_gravel", "setblock -2 -27 0 suspicious_sand", "give @s brush", ""]);
    lines.extend(["!au3brush 2 -1 0 35 30 60", "", "", "!au3brush -2 -1 0 12 35 12 25 80"].map(String::from));
    with(lines, &["gamemode creative @s", "clear @s", "kill @e[type=item]", "fill -2 -27 0 2 -27 0 stone", "fill -2 -28 0 2 -28 0 air"])
}

/// Runs the actor's own ticks for `ticks`, its packets unshown.
async fn run(actor: &mut acacia_bot::Bot, ticks: u32) {
    let end = tokio::time::Instant::now() + crate::probe::clock::scaled(Duration::from_millis(50 * u64::from(ticks)));
    while let Ok(Some(_)) = tokio::time::timeout_at(end, actor.next()).await {}
    tokio::time::sleep_until(end).await;
}

/// Does `!au3brush x y z held paused held ...`: the cell at the actor's feet plus x y z brushed from
/// above, held and let go for the tick counts given in turn, a held use each tick.
pub async fn act(actor: &mut acacia_bot::Bot, line: &str) -> bool {
    let Some(words) = line.strip_prefix("!au3brush ") else { return false };
    let numbers: Vec<i32> = words.split(' ').filter_map(|word| word.parse().ok()).collect();
    let (Some(feet), [x, y, z, spans @ ..]) = (actor.movement().and_then(|movement| movement.position()), &numbers[..]) else { return true };
    let pos = [feet[0].floor() as i32 + x, feet[1].floor() as i32 + y, feet[2].floor() as i32 + z];
    println!("## look: {:?}", actor.look_at([pos[0] as f32 + 0.5, pos[1] as f32 + 1.0, pos[2] as f32 + 0.5]).await);
    let face = acacia_bot::interact::Face::Up;
    for (i, &ticks) in spans.iter().enumerate() {
        if i % 2 == 1 {
            println!("## paused {ticks}");
            run(actor, ticks as u32).await;
            continue;
        }
        println!("## held {ticks}");
        super::brush_hold::press(actor, pos, face);
        for _ in 0..ticks {
            run(actor, 1).await;
            super::brush_hold::held(actor, pos, face);
        }
        super::brush_hold::release(actor, pos);
    }
    println!("## let go");
    true
}
