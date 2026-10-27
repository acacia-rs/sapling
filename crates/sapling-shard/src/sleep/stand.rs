//! Where a sleeper stands up, as BDS's `BedBlock::findStandUpPosition` (142cd9510) picks it: the
//! first of twelve cells around the bed's foot, turned with the bed, that has a floor, room for
//! the feet and air for the head, and is not fire or lava. Of two tables, the one for the side
//! of the bed the sleeper lay down from. Matched on BDS for an open spot, a walled one and a walled
//! ring (research/captures/bds-rest.txt).

use sapling_api::BlockPos;
use sapling_world::{BlockRegistry, World as Level};

/// Each table as BDS keeps it (14a7d6220, 14a7d62b0): x, y, z before the bed's turn.
const NEAR_SIDE: [[i32; 3]; 12] = [[1, 0, 0], [1, 0, 1], [1, 0, 2], [0, 0, 2], [-1, 0, 2], [-1, 0, 1], [-1, 0, 0], [-1, 0, -1], [0, 0, -1], [1, 0, -1], [0, 1, 0], [0, 1, 1]];
const FAR_SIDE: [[i32; 3]; 12] = [[-1, 0, 0], [-1, 0, 1], [-1, 0, 2], [0, 0, 2], [1, 0, 2], [1, 0, 1], [1, 0, 0], [1, 0, -1], [0, 0, -1], [-1, 0, -1], [0, 1, 0], [0, 1, 1]];
/// The turn BDS gives each bed `direction` (14a6820f4).
const TURN: [usize; 4] = [2, 3, 0, 1];
// TODO: BDS's own danger test reads a block flag and material 6 of feet and floor; fire and lava
// stand in for it here.
const DANGEROUS: [&str; 5] = ["minecraft:fire", "minecraft:soul_fire", "minecraft:lava", "minecraft:flowing_lava", "minecraft:magma"];

fn turned([x, y, z]: [i32; 3], turn: usize) -> [i32; 3] {
    match turn {
        1 => [-z, y, x],
        2 => [-x, y, -z],
        3 => [z, y, -x],
        _ => [x, y, z],
    }
}

/// Whether `entered` lies on the far side of the foot's middle for a bed facing `direction`.
fn far_side(direction: usize, foot: BlockPos, entered: [f32; 3]) -> bool {
    let middle = |axis: usize| foot[axis] as f32 + 0.5;
    match direction {
        0 => middle(0) < entered[0],
        1 => middle(2) < entered[2],
        2 => entered[0] < middle(0),
        _ => entered[2] < middle(2),
    }
}

fn name(registry: &BlockRegistry, id: u32) -> &str {
    registry.get(id).map_or("", |state| state.name)
}

/// The cell to stand in after sleeping in the bed whose foot is at `foot`, lain in from
/// `entered`: the first safe one, else the first with room, else none (BDS: `tile.bed.obstructed`).
pub(super) fn stand_up_cell(level: &Level, foot: BlockPos, direction: usize, entered: [f32; 3]) -> Option<BlockPos> {
    let registry = level.registry();
    let at = |[x, y, z]: BlockPos| level.block(x, y, z);
    let table = if far_side(direction, foot, entered) { FAR_SIDE } else { NEAR_SIDE };
    let mut roomy = None;
    for offset in table {
        let [dx, dy, dz] = turned(offset, TURN[direction % 4]);
        let cell = [foot[0] + dx, foot[1] + dy, foot[2] + dz];
        let (floor, feet, head) = (at([cell[0], cell[1] - 1, cell[2]]), at(cell), at([cell[0], cell[1] + 1, cell[2]]));
        let has_floor = registry.get(floor).is_some_and(|state| state.is_solid());
        let is_bed = name(registry, feet) == "minecraft:bed";
        let feet_free = is_bed || registry.get(feet).is_some_and(|state| !state.is_solid());
        let head_free = registry.get(head).is_some_and(|state| state.is_air()) && (!is_bed || registry.get(at([cell[0], cell[1] + 2, cell[2]])).is_some_and(|state| state.is_air()));
        if !(has_floor && feet_free && head_free) {
            continue;
        }
        if !DANGEROUS.contains(&name(registry, feet)) && !DANGEROUS.contains(&name(registry, floor)) {
            return Some(cell);
        }
        roomy.get_or_insert(cell);
    }
    roomy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bed_facing_south_stands_its_sleeper_where_bds_did() {
        // research/captures/bds-rest.txt: foot at x 1, head south of it, lain in from x 0.5.
        let order: Vec<[i32; 3]> = NEAR_SIDE.iter().map(|offset| turned(*offset, TURN[0])).take(3).collect();
        assert_eq!(order, [[-1, 0, 0], [-1, 0, -1], [-1, 0, -2]], "own cell, then one north, then two");
        assert!(!far_side(0, [1, 0, 0], [0.5, 0.0, 0.5]));
    }
}
