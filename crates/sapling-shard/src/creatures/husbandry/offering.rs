//! Where `behavior.drop_item_for` brings a mob before it drops its offering, as BDS's `DropItemForGoal`
//! (Linux ELF vtable 0xeee83c8: findTargetBlock 0x57754a0, isValidTarget 0x57757a0, tick 0x5773eb0; read
//! under gdb in research/captures/bds-ay2gdb.txt, docs/research/husbandry.md "Sleeping with its owner").
//! Around the block of the position (eyes) of the player it offers to it scans as `MoveToBlockGoal` does: layers one under the
//! player's block then up and down in turn, rings outwards, each ring's cells in Java's order. A cell
//! counts with a solid block and air over it; the first whose squared distance in blocks from the
//! player's is nearest `offering_distance` squared wins. A mob further than `minimum_teleport_distance`
//! (compared with the squared distance, as BDS does) from that block's corner is put on it at the corner
//! plus `teleport_offset`.

use sapling_api::BlockPos;
use sapling_world::World as Level;

/// The cat's (pack `cat.json`): `offering_distance` 5, `search_range` 5, `search_height` 2,
/// `minimum_teleport_distance` 2, `teleport_offset` (0, 1, 0).
const OFFERING_SQ: i32 = 25;
const RANGE: i32 = 5;
const HEIGHT: i32 = 2;
const TELEPORT_PAST: f32 = 2.0;
const OFFSET: [f32; 3] = [0.0, 1.0, 0.0];

/// 0, 1, -1, 2, -2 ... while within `limit`.
fn outwards(start: i32, limit: i32) -> impl Iterator<Item = i32> {
    std::iter::successors(Some(start), |&n| Some(if n > 0 { -n } else { 1 - n })).take_while(move |&n| n <= limit)
}

fn fits(level: &Level, [x, y, z]: BlockPos) -> bool {
    let state = |y: i32| level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten();
    state(y + 1).is_none_or(|over| over.is_air()) && state(y).is_some_and(|under| under.is_solid())
}

/// The block a mob offering to a player at `eyes` goes to: BDS scans round the block of the player's
/// position, its eyes (owner up at (-3.5, -24.38, 3.5) gave (-2, -27, 7)).
pub(super) fn spot(level: &Level, eyes: [f32; 3]) -> Option<BlockPos> {
    let [ox, oy, oz] = eyes.map(|c| c.floor() as i32);
    let mut best: Option<(i32, BlockPos)> = None;
    for dy in outwards(0, HEIGHT) {
        for ring in 0..RANGE {
            for dx in outwards(0, ring) {
                let first = if dx > -ring && dx < ring { ring } else { 0 };
                for dz in outwards(first, ring) {
                    let cell = [ox + dx, oy - 1 + dy, oz + dz];
                    if !fits(level, cell) {
                        continue;
                    }
                    let away = (cell[0] - ox).pow(2) + (cell[1] - oy).pow(2) + (cell[2] - oz).pow(2);
                    let score = (OFFERING_SQ - away).abs();
                    if best.is_none_or(|(kept, _)| score < kept) {
                        best = Some((score, cell));
                    }
                }
            }
        }
    }
    best.map(|(_, cell)| cell)
}

/// Where a mob at `at` is put to offer at `spot`, if it is that far off.
pub(super) fn teleport_to(at: [f32; 3], spot: BlockPos) -> Option<[f32; 3]> {
    let corner = spot.map(|c| c as f32);
    let away: f32 = (0..3).map(|axis| (corner[axis] - at[axis]).powi(2)).sum();
    (away > TELEPORT_PAST).then(|| std::array::from_fn(|axis| corner[axis] + OFFSET[axis]))
}
