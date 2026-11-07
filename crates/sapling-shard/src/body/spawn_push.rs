//! Where BDS puts a mob or boat `/summon`ed with its box in blocks, fitted to 45 of 47 summons
//! (research/captures/bds-ao2boat.txt, bds-ao2push.txt, bds-uuspawn.txt; the code was not found, see
//! docs/research/water-movement.md "Summoned into blocks"): each block box it overlaps at the start,
//! taken in x, then z, then y order and only while it still overlaps, pushes it out through that box's
//! nearest face, ties going +x, -x, +z, -z, +y, -y. One push may leave it in another block.

use acacia_physics::{Aabb, WorldView};
use sapling_world::World as Level;

use crate::terrain::physics_view;

/// The feet of a box `size` (width, height) summoned at `feet`, pushed out of the blocks it is in.
pub(crate) fn pushed_out(level: &Level, feet: [f32; 3], [width, height]: [f32; 2]) -> [f32; 3] {
    let half = width / 2.0;
    let mut lo = [feet[0] - half, feet[1], feet[2] - half];
    let mut hi = [feet[0] + half, feet[1] + height, feet[2] + half];
    let boxes: Vec<Aabb> = physics_view(level, |view| block_boxes(view, lo, hi)).into_iter().filter(|b| overlaps(lo, hi, b)).collect();
    for b in &boxes {
        if !overlaps(lo, hi, b) {
            continue;
        }
        let ways = [(0, b.max[0] - lo[0], 1.0), (0, hi[0] - b.min[0], -1.0), (2, b.max[2] - lo[2], 1.0), (2, hi[2] - b.min[2], -1.0), (1, b.max[1] - lo[1], 1.0), (1, hi[1] - b.min[1], -1.0)];
        let (axis, depth, sign) = ways.into_iter().fold(ways[0], |best, way| if way.1 < best.1 { way } else { best });
        lo[axis] += sign * depth;
        hi[axis] += sign * depth;
    }
    [lo[0] + half, lo[1], lo[2] + half]
}

/// [`pushed_out`] for a mob, then onto the top of a block it was pushed down into: BDS showed a cow, a
/// horse and a llama 0.3 to 0.87 into a roof block standing where summoned (bds-ao2push.txt).
pub(crate) fn pushed_out_standing(level: &Level, feet: [f32; 3], size: [f32; 2]) -> [f32; 3] {
    let mut body = super::Body::new(pushed_out(level, feet, size), [0.0; 3], size);
    body.rise_onto_low_block(level);
    body.at
}

/// The collision boxes of the blocks the box `lo`..`hi` reaches, in world coordinates, x then z then y.
fn block_boxes(view: &impl WorldView, lo: [f32; 3], hi: [f32; 3]) -> Vec<Aabb> {
    let span = |axis: usize| lo[axis].floor() as i32..=hi[axis].floor() as i32;
    let mut boxes = Vec::new();
    for x in span(0) {
        for z in span(2) {
            for y in span(1) {
                let mut local = Vec::new();
                view.block_collisions([x, y, z], &mut local);
                let at = [x as f32, y as f32, z as f32];
                boxes.extend(local.into_iter().map(|b| Aabb { min: std::array::from_fn(|i| b.min[i] + at[i]), max: std::array::from_fn(|i| b.max[i] + at[i]) }));
            }
        }
    }
    boxes
}

fn overlaps(lo: [f32; 3], hi: [f32; 3], b: &Aabb) -> bool {
    (0..3).all(|axis| lo[axis] < b.max[axis] && hi[axis] > b.min[axis])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floored(extra: &[[i32; 3]]) -> Level {
        let mut level = Level::void();
        let stone = level.block_id("minecraft:stone").unwrap();
        level.fill([-4, 63, -4], [8, 63, 8], stone);
        extra.iter().for_each(|&[x, y, z]| _ = level.set_block(x, y, z, stone));
        level
    }

    fn near(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3)
    }

    /// A boat's box is summoned 0.375 into the floor: whole edges go up, an edge 0.2 into a column goes out of it.
    #[test]
    fn a_boat_box_in_the_floor_leaves_its_low_columns_first_and_rises() {
        let level = floored(&[]);
        let boat = [1.4, 0.455];
        assert!(near(pushed_out(&level, [2.5, 63.625, 1.5], boat), [2.7, 64.0, 1.7]));
        assert!(near(pushed_out(&level, [0.35, 63.625, 0.85], boat), [0.7, 64.0, 0.85]));
        assert!(near(pushed_out(&level, [6.3, 63.625, 2.8], boat), [6.3, 64.0, 2.8]));
        assert!(near(pushed_out(&level, [3.5, 63.825, 4.5], boat), [3.5, 64.0, 4.5]), "0.175 in: up, not out");
    }

    /// A zombie under a roof block goes out of its side; a cow 0.3 into one is pushed down.
    #[test]
    fn a_mob_with_its_head_in_a_roof_block_goes_out_beside_it_or_down() {
        let level = floored(&[[2, 65, -6], [5, 65, -2], [12, 64, -1]]);
        assert!(near(pushed_out(&level, [2.5, 64.0, -5.5], [0.6, 1.95]), [3.3, 64.0, -5.5]));
        assert!(near(pushed_out(&level, [5.5, 64.0, -1.7], [0.6, 1.95]), [5.5, 64.0, -2.3]));
        assert!(near(pushed_out(&level, [2.5, 64.0, -5.5], [0.9, 1.3]), [2.5, 63.7, -5.5]));
        assert!(near(pushed_out(&level, [11.8, 64.0, -1.2], [0.9, 0.9]), [11.8, 64.0, -1.45]));
    }

    /// A cow pushed down into the floor stands back on it, where BDS showed it.
    #[test]
    fn a_mob_pushed_down_into_the_floor_stands_on_it() {
        let level = floored(&[[2, 65, 2]]);
        assert!(near(pushed_out_standing(&level, [2.5, 64.0, 2.5], [0.9, 1.3]), [2.5, 64.0, 2.5]));
    }
}
