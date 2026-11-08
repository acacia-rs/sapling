//! Where BDS puts a mob or boat `/summon`ed with its box in blocks: its initial-move depenetration, read
//! under gdb (docs/research/water-movement.md "Summoned into blocks"; solver 0x72b7190, per box 0xd7814b0).

use acacia_physics::{Aabb, WorldView};
use sapling_world::World as Level;

use crate::terrain::physics_view;

/// A block box deeper than this along its push axis is left alone (the boat's per-axis limit, 1.0).
const DEEPEST: f32 = 1.0;
/// Overlaps within this are none.
const SNAP: f32 = 1e-6;

/// The feet of a box `size` (width, height) summoned at `feet`, pushed out of the blocks it is in.
pub(crate) fn pushed_out(level: &Level, feet: [f32; 3], [width, height]: [f32; 2]) -> [f32; 3] {
    let half = width / 2.0;
    let mut lo = [feet[0] - half, feet[1], feet[2] - half];
    let mut hi = [feet[0] + half, feet[1] + height, feet[2] + half];
    let boxes = physics_view(level, |view| block_boxes(view, lo, hi));
    for _ in 0..3 {
        let mut shift = [0.0f32; 3];
        // every box is weighed against where the pass began; a push the other way replaces the axis's shift
        for b in boxes.iter().rev() {
            let Some((axis, push, depth)) = way_out(lo, hi, b) else { continue };
            if depth > DEEPEST {
                continue;
            }
            shift[axis] = if push > 0.0 { shift[axis].max(push) } else { shift[axis].min(push) };
        }
        for axis in 0..3 {
            lo[axis] += shift[axis];
            hi[axis] += shift[axis];
        }
    }
    [lo[0] + half, lo[1], lo[2] + half]
}

/// The axis, signed push and depth (the least overlap) out of `b`, if the box `lo`..`hi` is in it on every
/// axis: each axis takes its shallower side (a tie goes +), the box its shallowest axis (ties x, y, z).
/// Every f32 tie matters: (4.5, 4.5) and (9.5, 4.5) boats go opposite ways in z by one ulp.
fn way_out(lo: [f32; 3], hi: [f32; 3], b: &Aabb) -> Option<(usize, f32, f32)> {
    let snap = |v: f32| if v.abs() > SNAP { v } else { 0.0 };
    let mut depth = [0.0f32; 3];
    let mut sign = [1.0f32; 3];
    for axis in 0..3 {
        let (up, down) = (snap(b.max[axis] - lo[axis]), snap(hi[axis] - b.min[axis]));
        if up <= 0.0 || down <= 0.0 {
            return None;
        }
        depth[axis] = down.min(up);
        sign[axis] = if up > down { -1.0 } else { 1.0 };
    }
    let first = usize::from(depth[0] > depth[1]);
    let axis = if depth[first] <= depth[2] { first } else { 2 };
    Some((axis, depth[axis] * sign[axis], depth[0].min(depth[1]).min(depth[2])))
}

/// [`pushed_out`] for a mob, then onto the top of a block it was pushed down into: BDS showed a cow, a
/// horse and a llama 0.3 to 0.87 into a roof block standing where summoned (bds-ao2push.txt).
pub(crate) fn pushed_out_standing(level: &Level, feet: [f32; 3], size: [f32; 2]) -> [f32; 3] {
    let mut body = super::Body::new(pushed_out(level, feet, size), [0.0; 3], size);
    body.rise_onto_low_block(level);
    body.at
}

/// The collision boxes of the blocks the box `lo`..`hi` reaches, in world coordinates, x then z then y
/// (BDS's list order; its passes take them last first).
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

#[cfg(test)]
mod tests {
    use super::*;

    fn floored(extra: &[[i32; 3]]) -> Level {
        let mut level = Level::void();
        let stone = level.block_id("minecraft:stone").unwrap();
        level.fill([-4, 63, -4], [12, 63, 8], stone);
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

    /// At z 4.5 the z push's side follows f32 ties with x (bds-ba2boat.txt): x 4.5 and -0.5 go to 4.3, x 9.5 to 4.7.
    #[test]
    fn boats_at_z_four_and_a_half_go_the_way_bds_sent_them() {
        let level = floored(&[]);
        let boat = [1.4, 0.455];
        assert!(near(pushed_out(&level, [4.5, 63.625, 4.5], boat), [4.7, 64.0, 4.3]));
        assert!(near(pushed_out(&level, [9.5, 63.625, 4.5], boat), [9.7, 64.0, 4.7]));
        assert!(near(pushed_out(&level, [-0.5, 63.625, 4.5], boat), [-0.3, 64.0, 4.3]));
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
