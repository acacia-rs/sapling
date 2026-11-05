//! The thirteenth wave's group A2 drills (research/parity/THIRTEENTH-BRIEF.md): how many ticks after a
//! command a block tick it queues runs (`e2clock`; docs/research/tick-queue.md "the chunk clock").

use super::creatures::{floor, waits};

const CLOCK_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "CommandOutput"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "e2clock" => (clock(), CLOCK_SHOWN),
        "e2stale" => (stale(), CLOCK_SHOWN),
        "e2rails" => (rails(), RAILS_SHOWN),
        "e2lids" => (lids(), LIDS_SHOWN),
        _ => return None,
    })
}

const LIDS_SHOWN: &[&str] = &["ContainerOpen", "ContainerClose", "BlockEvent", "CommandOutput", "UpdateBlock"];

/// Shulker boxes clicked open: one with stone over it, one with a bottom slab over it, one under a
/// carpet, one with nothing over it.
fn lids() -> Vec<String> {
    let mut lines = floor();
    for (z, over) in [(-2, "stone"), (0, "oak_slab"), (2, "white_carpet"), (4, "air")] {
        lines.extend([format!("setblock 2 -26 {z} shulker_box"), format!("setblock 2 -25 {z} {over}")]);
    }
    waits(&mut lines, 1);
    for z in [-2, 0, 2, 4] {
        lines.extend([format!("!click 2 0 {z} west"), String::new(), "!close".to_string(), String::new()]);
    }
    lines
}

const RAILS_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "CommandOutput", "AddEntity", "AddItemEntity", "RemoveEntity", "TakeItemActor", "MoveEntityDelta", "MoveEntityAbsolute"];

/// A detector rail pressed by a summoned minecart and let go when it is killed; a hopper minecart
/// standing on a rail at x 0.5, z -5.5 and dirt dropped 0.75, 1, 1.25 and 1.5 east of it and 1.5 above
/// it; a second one on a powered activator rail at x 4.5, given dirt, then the power taken away.
fn rails() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["setblock 8 -26 -8 dirt", "setblock 2 -26 6 detector_rail", "summon minecart 2.5 -26 6.5"].map(String::from));
    waits(&mut lines, 3);
    lines.push("kill @e[type=minecart]".to_string());
    waits(&mut lines, 3);
    lines.extend(["fill -1 -26 -7 6 -26 -7 stone", "fill -1 -26 -4 6 -26 -4 stone", "setblock 0 -26 -6 rail", "setblock 4 -27 -6 redstone_block", "setblock 4 -26 -6 activator_rail"].map(String::from));
    lines.extend(["summon hopper_minecart 0.5 -26 -5.5", "summon hopper_minecart 4.5 -26 -5.5"].map(String::from));
    waits(&mut lines, 2);
    for x in ["1.25", "1.5", "1.75", "2.0"] {
        lines.push(format!("loot spawn {x} -26 -5.5 mine 8 -26 -8"));
        waits(&mut lines, 2);
    }
    lines.push("loot spawn 0.5 -24.5 -5.5 mine 8 -26 -8".to_string());
    waits(&mut lines, 3);
    lines.push("loot spawn 4.5 -26 -5.5 mine 8 -26 -8".to_string());
    waits(&mut lines, 3);
    lines.push("setblock 4 -27 -6 stone".to_string());
    waits(&mut lines, 3);
    lines.push("kill @e[type=hopper_minecart]".to_string());
    lines
}

/// Chunk 3, 0 (whose diamond holds chunk 4, 2), chunk 0, 0 (whose diamond holds 4, 1 but not 4, 2) and
/// chunk 3, 1 (whose diamond holds 4, 2), as in seventh2's `tickqueue`.
const SETTING: &str = "52.5 -60 0.5";
const AWAY: &str = "8.5 -60 0.5";
const BACK: &str = "52.5 -60 24.5";

/// Lava at x 66, z 30 in chunk 4, 1, plugged at z 31, runs into 4, 2 (z 32 on, written 3 s after the plug
/// goes) while the bots are at chunk 0, 0, where 4, 2 does not tick; they come back a second later. If
/// 4, 2's queue keeps its own clock (BDS `mCurrentTick`, the last tick it ran), the tick z 32 queued is
/// long due and z 33 follows as they arrive; on the level's clock it waits out its 30 ticks (4.5 s).
fn stale() -> Vec<String> {
    let mut lines = floor();
    lines.extend([format!("tp @s {SETTING}"), format!("tp CmdWatcher {SETTING}")]);
    waits(&mut lines, 3);
    lines.extend([
        "fill 65 -32 29 67 -31 41 stone".to_string(),
        "fill 66 -31 32 66 -31 41 air".to_string(),
        "setblock 66 -31 30 lava".to_string(),
    ]);
    waits(&mut lines, 1);
    lines.extend([format!("tp @s {AWAY}"), format!("tp CmdWatcher {AWAY}")]);
    waits(&mut lines, 4);
    lines.push("setblock 66 -31 31 air".to_string());
    waits(&mut lines, 3);
    lines.extend([format!("tp @s {BACK}"), format!("tp CmdWatcher {BACK}")]);
    waits(&mut lines, 6);
    lines
}

/// Rows of the clock drill: water at z -6, -2 and 2, lava at 6.
const ROWS: [(i32, &str); 4] = [(-6, "water"), (-2, "water"), (2, "water"), (6, "lava")];

/// Per row a liquid source at x 2 walled in, a plug at x 3, an open run from x 4 to 12. A second after
/// each plug is set to air, the liquid's first step (BDS `neighborChanged` queues it) is timed against the
/// command's answer and against its next step, which its own tick queued.
fn clock() -> Vec<String> {
    let mut lines = floor();
    for (z, liquid) in ROWS {
        lines.extend([
            format!("fill 1 -26 {} 12 -26 {} stone", z - 1, z + 1),
            format!("fill 4 -26 {z} 12 -26 {z} air"),
            format!("setblock 2 -26 {z} {liquid}"),
        ]);
    }
    waits(&mut lines, 2);
    for (z, liquid) in ROWS {
        lines.push(format!("setblock 3 -26 {z} air"));
        waits(&mut lines, if liquid == "lava" { 5 } else { 2 });
    }
    lines
}
