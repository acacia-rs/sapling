//! Drills of the fifth follow-up wave's group 2 (docs/research mob-damage.md, structure-block-entities.md):
//! where a command may change blocks about the bots at `tick-distance` 4, and what BDS sends for a
//! block with an entity set by command.

use super::creatures::{floor, waits, with};

const EDGE_SHOWN: &[&str] = &["UpdateBlock"];
const SET_SHOWN: &[&str] = &["UpdateBlock", "BlockEntityData"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "tickedge" => (tick_edge(), EDGE_SHOWN),
        "setshown" => (set_shown(), SET_SHOWN),
        _ => return None,
    })
}

/// Chunk offsets from the bots' chunk (3, 0) past the quadrant swept: the other quadrants' edges.
const OTHER_SIDES: [(i32, i32); 10] = [(-4, 0), (-5, 0), (0, -4), (0, -5), (-3, -2), (-3, -3), (-2, -3), (4, -1), (-1, 4), (4, -2)];

/// The middle block's x and z of the chunk `dx`, `dz` off the bots'.
fn centre(dx: i32, dz: i32) -> (i32, i32) {
    ((3 + dx) * 16 + 8, dz * 16 + 8)
}

/// One-block fills of air in air at every chunk offset 0..=6 by 0..=6 and on the other sides, then
/// `/setblock`, `/clone`, `/testforblock` and `/testforblocks` into the first chunk past the edge,
/// and fills that reach over the edge: which answer `outOfWorld`.
fn tick_edge() -> Vec<String> {
    let mut lines = with(floor(), &["tp @s 52.5 -60 0.5", "tp CmdWatcher 52.5 -60 2.5"]);
    waits(&mut lines, 3);
    let offsets = (0..=6).flat_map(|dx| (0..=6).map(move |dz| (dx, dz))).chain(OTHER_SIDES);
    lines.extend(offsets.map(|(dx, dz)| {
        let (x, z) = centre(dx, dz);
        format!("fill {x} -40 {z} {x} -40 {z} air")
    }));
    let ((inside, z), (outside, _)) = (centre(4, 0), centre(5, 0));
    let last = (3 + 4) * 16 + 15;
    lines.extend([
        format!("setblock {inside} -40 {z} stone"),
        format!("setblock {outside} -40 {z} stone"),
        format!("testforblock {inside} -40 {z} stone"),
        format!("testforblock {outside} -40 {z} stone"),
        format!("clone {inside} -40 {z} {inside} -40 {z} {outside} -40 {z}"),
        format!("clone {outside} -40 {z} {outside} -40 {z} {inside} -41 {z}"),
        format!("testforblocks {inside} -40 {z} {inside} -40 {z} {outside} -40 {z}"),
        format!("fill {last} -40 {z} {last} -40 {z} stone"),
        format!("fill {last} -40 {z} {} -40 {z} stone", last + 1),
        format!("fill {inside} -40 {z} {inside} -40 {z} air"),
        format!("fill {last} -40 {z} {last} -40 {z} air"),
        String::new(),
    ]);
    lines.extend(with(Vec::new(), &["tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""]));
    lines
}

/// Blocks with entities set by command in a row east of the actor, twice: on air and over
/// themselves in another state.
fn set_shown() -> Vec<String> {
    let blocks = [
        "flower_pot",
        "cauldron",
        "skeleton_skull [\"facing_direction\"=1]",
        "beacon",
        "chiseled_bookshelf",
        "bed",
        "bell",
        "chest",
        "frame",
    ];
    let mut lines = floor();
    for (z, block) in (-8..).step_by(2).zip(blocks) {
        lines.push(format!("setblock 2 -26 {z} {block}"));
    }
    lines.extend(["setblock 2 -26 -6 cauldron [\"cauldron_liquid\"=\"water\",\"fill_level\"=3]".to_owned(), String::new()]);
    lines.extend((-8..).step_by(2).take(blocks.len()).map(|z| format!("setblock 2 -26 {z} air")));
    lines.push(String::new());
    lines
}
