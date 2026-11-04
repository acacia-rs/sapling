//! Drills of the sixth follow-up wave's group 2 (docs/research/growth.md): which columns about the bots
//! at `tick-distance` 4 get random ticks, a ticking area's included.

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &["UpdateBlock"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "tickrandom" => (tick_random(), SHOWN),
        _ => return None,
    })
}

/// Chunk offsets from the bots' chunk (0, 0) while the rule runs: inside the tick diamond (4,1 is past
/// the circle dx² + dz² <= 16), outside it (4,2 is inside the square), and 6,0 in a ticking area.
const OFFSETS: [(i32, i32); 14] = [(0, 0), (0, 1), (4, 0), (4, 1), (3, 2), (2, 3), (2, -3), (5, 0), (5, 1), (4, 2), (3, 3), (3, -3), (6, 1), (6, 0)];
const KEPT: (i32, i32) = (6, 0);
/// Where the bots stand while the cells are set: chunk 3, 0, whose diamond holds every offset.
const SETTING: &str = "52.5 -60 0.5";
const RUNNING: &str = "8.5 -60 0.5";

/// The first cell's x and z of the row at chunk offset `dx`, `dz`.
fn corner(dx: i32, dz: i32) -> (i32, i32) {
    (dx * 16 + 2, dz * 16 + 2)
}

/// Per offset a wheat on farmland, bare dry farmland, a copper block and oak planks with fire on
/// them, set from chunk 3, 0 with the rule at 0; the bots go to chunk 0, 0, the rule is 200 for 20 s,
/// then 0 again and each cell is read from chunk 3, 0.
fn tick_random() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule randomtickspeed 0", "gamerule dofiretick true", "time set noon"]);
    lines.extend([format!("tp @s {SETTING}"), format!("tp CmdWatcher {SETTING}")]);
    waits(&mut lines, 3);
    for (dx, dz) in OFFSETS {
        let (x, z) = corner(dx, dz);
        lines.extend([
            format!("fill {x} -61 {z} {} -59 {z} air", x + 9),
            format!("setblock {x} -61 {z} farmland"),
            format!("setblock {x} -60 {z} wheat"),
            format!("setblock {} -61 {z} farmland", x + 3),
            format!("setblock {} -60 {z} copper_block", x + 6),
            format!("setblock {} -60 {z} oak_planks", x + 9),
        ]);
    }
    let (kx, kz) = (KEPT.0 * 16, KEPT.1 * 16);
    lines.push(format!("tickingarea add {kx} -64 {kz} {} -50 {} randomkept", kx + 15, kz + 15));
    lines.extend(OFFSETS.map(|(dx, dz)| {
        let (x, z) = corner(dx, dz);
        format!("setblock {} -59 {z} fire", x + 9)
    }));
    lines.extend([format!("tp @s {RUNNING}"), format!("tp CmdWatcher {RUNNING}")]);
    waits(&mut lines, 2);
    lines.push("gamerule randomtickspeed 200".to_owned());
    waits(&mut lines, 20);
    lines.extend(["gamerule randomtickspeed 0", "gamerule dofiretick false", "tickingarea remove randomkept"].map(String::from));
    lines.extend([format!("tp @s {SETTING}"), format!("tp CmdWatcher {SETTING}")]);
    waits(&mut lines, 3);
    for (dx, dz) in OFFSETS {
        let (x, z) = corner(dx, dz);
        lines.extend([
            format!("testforblock {x} -60 {z} wheat [\"growth\"=0]"),
            format!("testforblock {} -61 {z} farmland", x + 3),
            format!("testforblock {} -60 {z} copper_block", x + 6),
            format!("testforblock {} -60 {z} oak_planks", x + 9),
            format!("testforblock {} -59 {z} fire", x + 9),
        ]);
    }
    for (dx, dz) in OFFSETS {
        let (x, z) = corner(dx, dz);
        lines.push(format!("fill {x} -61 {z} {} -59 {z} air", x + 9));
    }
    lines.extend(["gamerule randomtickspeed 1", "gamerule dofiretick true", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""].map(String::from));
    lines
}
