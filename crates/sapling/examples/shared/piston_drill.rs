//! The slime drill of bds_command_probe: pistons with slime and honey (docs/research/redstone.md).

const EAST: &str = "piston [\"facing_direction\"=4]";
const STICKY: &str = "sticky_piston [\"facing_direction\"=4]";

/// Pistons hung in the air and pushing east, each powered from below and let go: slime taking
/// a block behind it, beside it, an obsidian, a honey block; honey; 12 blocks and 13; one held
/// back; glazed terracotta and a chest; a sticky one drawing slime and what slime holds.
pub fn slime() -> Vec<String> {
    let mut lines = vec!["fill ~2 ~-1 ~4 ~22 ~-1 ~16 stone".to_owned(), "fill ~2 ~ ~4 ~22 ~8 ~16 air".to_owned(), "fill ~2 ~9 ~4 ~22 ~16 ~16 air".to_owned()];
    let line = |from: i32, to: i32| (from..=to).map(|x| (x, 0, 0, "stone"));
    let rows: [(i32, i32, &str, Vec<(i32, i32, i32, &str)>); 12] = [
        (2, 5, EAST, vec![(5, 0, 0, "slime"), (6, 0, 0, "stone")]),
        (2, 8, EAST, vec![(5, 0, 0, "slime"), (5, 1, 0, "stone"), (5, 0, 1, "stone"), (5, -1, 0, "stone")]),
        (2, 11, EAST, vec![(5, 0, 0, "slime"), (5, 1, 0, "obsidian")]),
        (2, 14, STICKY, vec![(5, 0, 0, "slime"), (6, 0, 0, "stone"), (5, 1, 0, "stone")]),
        (7, 5, EAST, vec![(5, 0, 0, "slime"), (5, 1, 0, "honey_block"), (5, 2, 0, "stone")]),
        (7, 8, EAST, vec![(5, 0, 0, "honey_block"), (5, 1, 0, "stone"), (6, 0, 0, "stone")]),
        (7, 11, EAST, [(5, 0, 0, "slime"), (5, 1, 0, "stone")].into_iter().chain(line(6, 15)).collect()),
        (7, 14, EAST, [(5, 0, 0, "slime"), (5, 1, 0, "stone")].into_iter().chain(line(6, 16)).collect()),
        (12, 5, EAST, vec![(5, 0, 0, "slime"), (5, 1, 0, "stone"), (6, 1, 0, "obsidian")]),
        (12, 8, EAST, vec![(5, 0, 0, "slime"), (5, 1, 0, "white_glazed_terracotta"), (5, 0, 1, "chest")]),
        (12, 11, STICKY, vec![(5, 0, 0, "stone"), (6, 0, 0, "slime"), (6, 1, 0, "stone")]),
        (12, 14, EAST, vec![(5, 0, 0, "slime"), (5, 1, 0, "slime"), (5, 2, 0, "stone"), (6, 1, 0, "stone")]),
    ];
    for (y, z, piston, pushed) in rows {
        lines.push(format!("setblock ~4 ~{y} ~{z} {piston}"));
        lines.extend(pushed.into_iter().map(|(x, up, south, block)| format!("setblock ~{x} ~{} ~{} {block}", y + up, z + south)));
        lines.extend(["redstone_block", "air"].map(|block| format!("setblock ~4 ~{} ~{z} {block}", y - 1)));
    }
    lines
}
