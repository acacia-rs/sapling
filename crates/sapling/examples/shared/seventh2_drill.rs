//! Drills of the seventh follow-up wave's group 2: the scheduled-block queue past the tick diamond
//! (docs/research/growth.md "Where random ticks run") and fire and lit ore at a few `randomTickSpeed`
//! values (docs/research/fire.md).

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &["UpdateBlock"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "tickqueue" => (tick_queue(), SHOWN),
        "firepace" => (fire_pace(), SHOWN),
        "hivekept" => (hive_kept(), SHOWN),
        _ => return None,
    })
}

/// Chunk offsets from the bots' chunk (0, 0) once the rigs are set off: in the diamond (4,0, 4,1, past the
/// circle), out of it (4,2, 5,0, 5,1) and 6,0 in a ticking area. Every one is in chunk 3, 0's diamond, the
/// box of them too, so one `fill` from there reaches them all: commands answer `testforblock.out` past
/// the diamond (bds-tickqueue1.txt).
const OFFSETS: [(i32, i32); 6] = [(4, 0), (4, 1), (4, 2), (5, 0), (5, 1), (6, 0)];
const KEPT: (i32, i32) = (6, 0);
/// Chunk 3, 0, whose diamond holds every offset, and chunk 0, 0.
const SETTING: &str = "52.5 -60 0.5";
const RUNNING: &str = "8.5 -60 0.5";
/// The layer whose plugs the trigger takes out; nothing else is in it.
const PLUGS: i32 = -30;

fn corner(dx: i32, dz: i32) -> (i32, i32) {
    (dx * 16 + 2, dz * 16 + 2)
}

/// Per offset, three rigs held by a plug in the layer y -30, set from chunk 3, 0 (row z + 1): a redstone
/// block over dust at x feeding six repeaters at their longest and a lamp (x + 7), all on stone at -32; a
/// lava source on a stone at x + 10; sand on a stone over a shaft of air down to the ground (x, z + 9).
/// One `fill` takes every plug out and the bots go to chunk 0, 0 a second later, before the lava's 30 ticks,
/// the repeaters' 48 and the sand's fall are done; five seconds on they come back and each cell is read.
fn tick_queue() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule randomtickspeed 0", "time set noon"]);
    lines.extend([format!("tp @s {SETTING}"), format!("tp CmdWatcher {SETTING}")]);
    waits(&mut lines, 3);
    let repeater = "unpowered_repeater [\"minecraft:cardinal_direction\"=\"west\",\"repeater_delay\"=3]";
    for (dx, dz) in OFFSETS {
        let (x, z) = corner(dx, dz);
        let (row, sand) = (z + 1, z + 9);
        lines.extend([
            format!("fill {x} -33 {z} {} -28 {sand} air", x + 12),
            format!("fill {x} -32 {row} {} -32 {row} stone", x + 8),
            format!("setblock {x} -31 {row} redstone_wire"),
            format!("fill {} -31 {row} {} -31 {row} {repeater}", x + 1, x + 6),
            format!("setblock {} -31 {row} redstone_lamp", x + 7),
            format!("setblock {x} {PLUGS} {row} redstone_block"),
            format!("setblock {} {PLUGS} {row} stone", x + 10),
            format!("setblock {} {} {row} lava", x + 10, PLUGS + 1),
            format!("fill {x} -60 {sand} {x} {} {sand} air", PLUGS - 1),
            format!("setblock {x} {PLUGS} {sand} stone"),
            format!("setblock {x} {} {sand} sand", PLUGS + 1),
        ]);
    }
    let (kx, kz) = (KEPT.0 * 16, KEPT.1 * 16);
    lines.push(format!("tickingarea add {kx} -64 {kz} {} -20 {} queuekept", kx + 15, kz + 15));
    waits(&mut lines, 4);
    let reads = |lines: &mut Vec<String>| {
        for (dx, dz) in OFFSETS {
            let (x, z) = corner(dx, dz);
            let row = z + 1;
            lines.extend([
                format!("testforblock {} -31 {row} lit_redstone_lamp", x + 7),
                format!("testforblock {} {} {row} air", x + 10, PLUGS - 1),
                format!("testforblock {x} -60 {} sand", z + 9),
            ]);
        }
    };
    reads(&mut lines);
    let (x0, z0) = (OFFSETS.iter().map(|o| o.0).min().unwrap_or(0) * 16, OFFSETS.iter().map(|o| o.1).min().unwrap_or(0) * 16);
    let (x1, z1) = (OFFSETS.iter().map(|o| o.0).max().unwrap_or(0) * 16 + 15, OFFSETS.iter().map(|o| o.1).max().unwrap_or(0) * 16 + 15);
    lines.extend([format!("fill {x0} {PLUGS} {z0} {x1} {PLUGS} {z1} air"), format!("tp @a {RUNNING}")]);
    waits(&mut lines, 5);
    lines.push(format!("tp @a {SETTING}"));
    waits(&mut lines, 4);
    reads(&mut lines);
    lines.push("tickingarea remove queuekept".to_owned());
    for (dx, dz) in OFFSETS {
        let (x, z) = corner(dx, dz);
        lines.extend([format!("fill {x} -33 {z} {} -28 {} air", x + 12, z + 9), format!("fill {x} -60 {} {x} -34 {} air", z + 9, z + 9)]);
    }
    lines.extend(["gamerule randomtickspeed 1", "tp @a 0.5 -26 0.5", ""].map(String::from));
    lines
}

/// Hives at chunk offsets 5,0 (out of the diamond) and 6,0 (a ticking area), two bees sent into each by
/// the night from chunk 3, 0; from chunk 0, 0 it is noon for 45 s, then the bees about each hive are
/// counted there and again back in chunk 3, 0.
fn hive_kept() -> Vec<String> {
    let hives = [(5, 0), (6, 0)].map(|(dx, dz)| corner(dx, dz));
    let mut lines = with(floor(), &["time set midnight"]);
    lines.extend([format!("tp @s {SETTING}"), format!("tp CmdWatcher {SETTING}")]);
    waits(&mut lines, 3);
    for (x, z) in hives {
        lines.extend([
            format!("fill {} -60 {} {} -55 {} air", x - 3, z - 3, x + 3, z + 4),
            format!("setblock {x} -60 {z} beehive [\"direction\"=3]"),
            format!("summon bee {}.5 -59 {}.5", x, z + 2),
            format!("summon bee {}.5 -58 {}.5", x + 1, z + 2),
        ]);
    }
    waits(&mut lines, 10);
    let count = |lines: &mut Vec<String>| {
        for (x, z) in hives {
            lines.push(format!("testfor @e[type=bee,x={x},y=-60,z={z},r=8]"));
        }
    };
    count(&mut lines);
    let (kx, kz) = (KEPT.0 * 16, KEPT.1 * 16);
    lines.push(format!("tickingarea add {kx} -64 {kz} {} -50 {} hivekept", kx + 15, kz + 15));
    lines.extend([format!("tp @s {RUNNING}"), format!("tp CmdWatcher {RUNNING}")]);
    waits(&mut lines, 2);
    lines.push("time set noon".to_owned());
    waits(&mut lines, 45);
    count(&mut lines);
    lines.extend([format!("tp @s {SETTING}"), format!("tp CmdWatcher {SETTING}")]);
    waits(&mut lines, 5);
    count(&mut lines);
    lines.extend(["tickingarea remove hivekept", "kill @e[type=bee]"].map(String::from));
    for (x, z) in hives {
        lines.push(format!("setblock {x} -60 {z} air"));
    }
    lines.extend(["tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""].map(String::from));
    lines
}

/// The rule values and how long each is watched, in seconds.
const PACES: [(u32, usize); 3] = [(1, 60), (4, 25), (20, 15)];

/// On the bots' platform, for each `randomTickSpeed` of [`PACES`]: twelve lone oak planks (z -6 and -2,
/// four apart) set first, then the rule, a fire on each plank, a row of twelve lit redstone ores in the
/// floor (z 3) and a row of twelve fires over stone (z 6); then what is left is read and the platform
/// cleared. Each cell's times are in its `UpdateBlock`s.
fn fire_pace() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule randomtickspeed 0", "gamerule dofiretick true", "time set noon", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5"]);
    let planks: Vec<(i32, i32)> = [-6, -2].into_iter().flat_map(|z| (0..6).map(move |i| (-6 + 4 * i, z))).collect();
    for (speed, seconds) in PACES {
        lines.extend(["fill -8 -26 -8 16 -20 8 air", "fill -8 -27 -8 16 -27 8 stone"].map(String::from));
        for &(x, z) in &planks {
            lines.push(format!("setblock {x} -26 {z} oak_planks"));
        }
        lines.push(format!("gamerule randomtickspeed {speed}"));
        for &(x, z) in &planks {
            lines.push(format!("setblock {x} -25 {z} fire"));
        }
        lines.extend(["fill -6 -27 3 5 -27 3 lit_redstone_ore", "fill -6 -26 6 5 -26 6 fire"].map(String::from));
        waits(&mut lines, seconds);
        lines.push("gamerule randomtickspeed 0".to_owned());
        lines.extend(["testforblock -6 -26 6 fire", "testforblock -6 -27 3 lit_redstone_ore"].map(String::from));
        for &(x, z) in &planks {
            lines.push(format!("testforblock {x} -26 {z} oak_planks"));
        }
    }
    lines.extend(["fill -8 -26 -8 16 -20 8 air", "fill -8 -27 -8 16 -27 8 stone", "gamerule randomtickspeed 1", ""].map(String::from));
    lines
}
