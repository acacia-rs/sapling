//! The eighth wave's block drills (research/parity/EIGHTH-BRIEF.md "Group 3"): hopper chains through
//! chests, one running east and its mirror west, let go in one command (`e8hopchain`); comparators
//! set again by `/setblock` in their own and in the other block type (`e8cmpset`); a jukebox filled
//! and emptied by hoppers and its signal through a short song (`e8juke`). On creature_drill.rs's
//! floor (stone at y -27).

use super::creatures::{floor, waits};

const SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "LevelEvent", "LevelSoundEvent", "BlockEntityData", "AddItemEntity", "Text"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "e8hopchain" => (hopper_chains(), SHOWN),
        "e8cmpset" => (comparators_set(), SHOWN),
        "e8juke" => (jukebox(), SHOWN),
        _ => return None,
    })
}

fn comparator(block: &str, rear: &str, subtract: bool, lit: bool) -> String {
    format!("{block} [\"minecraft:cardinal_direction\"=\"{rear}\",\"output_subtract_bit\"={subtract},\"output_lit_bit\"={lit}]")
}

/// A chain of four stages stepping down from y -22: a hopper pushing `step` along x into a chest,
/// the hopper under that chest pushing on, into a last chest a comparator reads. Its first hopper
/// holds one item, held by a power block over it.
fn chain(lines: &mut Vec<String>, start: i32, step: i32, z: i32) {
    let facing = if step > 0 { 5 } else { 4 };
    let rear = if step > 0 { "west" } else { "east" };
    lines.push(format!("setblock {start} -21 {z} redstone_block"));
    lines.push(format!("setblock {start} -22 {z} hopper [\"facing_direction\"={facing},\"toggle_bit\"=true]"));
    for stage in 1..=4 {
        let (x, y) = (start + step * stage, -22 - stage);
        lines.push(format!("setblock {x} {} {z} chest", y + 1));
        lines.push(format!("setblock {x} {y} {z} hopper [\"facing_direction\"={facing}]"));
    }
    let end = start + step * 5;
    lines.push(format!("setblock {end} -26 {z} chest"));
    lines.push(format!("setblock {} -26 {z} {}", end + step, comparator("unpowered_comparator", rear, false, false)));
    lines.push(format!("setblock {} -26 {z} redstone_wire", end + 2 * step));
    lines.push(format!("replaceitem block {start} -22 {z} slot.container 0 stone 1"));
}

/// Both chains let go by one fill of their power blocks; BDS ticks the hopper under a chest a hopper
/// pushes into first, so the east chain (sapling's sorted order runs it upstream first) shows whether
/// an item goes on in the tick it arrived.
fn hopper_chains() -> Vec<String> {
    let mut lines = floor();
    chain(&mut lines, 2, 1, -4);
    chain(&mut lines, 14, -1, 4);
    waits(&mut lines, 2);
    lines.push("fill 2 -21 -4 14 -21 4 air".into());
    waits(&mut lines, 5);
    lines
}

/// A comparator at (x, -26, z) reading 15 from dust behind it (west) and 15 from dust at its south
/// side, dust and a lamp east of it.
fn comparator_rig(lines: &mut Vec<String>, x: i32, z: i32, subtract: bool) {
    lines.extend([
        format!("setblock {} -26 {z} redstone_wire", x - 1),
        format!("setblock {x} -26 {} redstone_wire", z + 1),
        format!("setblock {x} -26 {z} {}", comparator("unpowered_comparator", "west", subtract, false)),
        format!("setblock {} -26 {z} redstone_wire", x + 1),
        format!("setblock {} -26 {z} redstone_lamp", x + 2),
        format!("setblock {} -26 {z} redstone_block", x - 2),
        format!("setblock {x} -26 {} redstone_block", z + 2),
    ]);
}

/// Rigs lit (compare, 15 at both inputs) and unlit (subtract), each comparator then set again by
/// `/setblock` to the other mode in its own block type or the other; one taken out and set anew;
/// one reading a full hopper set from subtract to compare. Then a block put over two of them.
fn comparators_set() -> Vec<String> {
    let mut lines = floor();
    let rigs = [(4, -7, false), (4, -3, false), (4, 1, true), (4, 5, true), (11, -7, true)];
    for (x, z, subtract) in rigs {
        comparator_rig(&mut lines, x, z, subtract);
    }
    lines.extend([
        "setblock 10 -26 1 hopper [\"facing_direction\"=1]".to_owned(),
        "replaceitem block 10 -26 1 slot.container 0 stone 64".to_owned(),
        "replaceitem block 10 -26 1 slot.container 1 stone 64".to_owned(),
        "replaceitem block 10 -26 1 slot.container 2 stone 64".to_owned(),
        "replaceitem block 10 -26 1 slot.container 3 stone 64".to_owned(),
        "replaceitem block 10 -26 1 slot.container 4 stone 64".to_owned(),
        "setblock 11 -26 2 redstone_wire".to_owned(),
        format!("setblock 11 -26 1 {}", comparator("unpowered_comparator", "west", true, false)),
        "setblock 12 -26 1 redstone_wire".to_owned(),
        "setblock 13 -26 1 redstone_lamp".to_owned(),
        "setblock 11 -26 3 redstone_block".to_owned(),
    ]);
    waits(&mut lines, 2);
    lines.extend([
        format!("setblock 4 -26 -7 {}", comparator("powered_comparator", "west", true, true)),
        String::new(),
        format!("setblock 4 -26 -3 {}", comparator("unpowered_comparator", "west", true, false)),
        String::new(),
        format!("setblock 4 -26 1 {}", comparator("unpowered_comparator", "west", false, false)),
        String::new(),
        format!("setblock 4 -26 5 {}", comparator("powered_comparator", "west", false, true)),
        String::new(),
        "setblock 11 -26 -7 air".to_owned(),
        format!("setblock 11 -26 -7 {}", comparator("unpowered_comparator", "west", false, false)),
        String::new(),
        format!("setblock 11 -26 1 {}", comparator("unpowered_comparator", "west", false, false)),
        String::new(),
    ]);
    waits(&mut lines, 2);
    lines.extend(["setblock 4 -25 1 glass", "", "setblock 11 -25 1 glass", "", ""].map(String::from));
    lines
}

/// A hopper over a jukebox holding disc 11 (71 s), a hopper under it into a chest; a lamp beside the
/// jukebox, comparators reading it and the chest. A second jukebox fed from the side by a hopper
/// holding stone before disc 13. Watched past the song's end.
fn jukebox() -> Vec<String> {
    let mut lines = floor();
    lines.extend([
        "setblock 4 -26 -4 hopper [\"facing_direction\"=5]",
        "setblock 5 -26 -4 chest",
        "setblock 6 -26 -4 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\"]",
        "setblock 7 -26 -4 redstone_wire",
        "setblock 4 -25 -4 jukebox",
        "setblock 3 -25 -4 redstone_lamp",
        "setblock 4 -26 -3 stone",
        "setblock 4 -26 -2 stone",
        "setblock 4 -25 -3 unpowered_comparator [\"minecraft:cardinal_direction\"=\"north\"]",
        "setblock 4 -25 -2 redstone_wire",
        "setblock 4 -24 -4 hopper [\"facing_direction\"=0]",
        "setblock 10 -26 4 jukebox",
        "setblock 11 -26 4 unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\"]",
        "setblock 12 -26 4 redstone_wire",
        "setblock 9 -26 4 hopper [\"facing_direction\"=5,\"toggle_bit\"=true]",
        "setblock 9 -25 4 redstone_block",
        "replaceitem block 9 -26 4 slot.container 0 stone 1",
        "replaceitem block 9 -26 4 slot.container 1 music_disc_13 1",
        "setblock 9 -25 4 air",
        "",
        "replaceitem block 4 -24 -4 slot.container 0 music_disc_11 1",
    ]
    .map(String::from));
    waits(&mut lines, 80);
    lines
}
