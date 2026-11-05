//! Where a flying shot meets a block: BDS casts its move as a ray from the shot's position through the
//! blocks' collision boxes (`ProjectileComponent`), so an arrow stops on the face it hit, not its box's
//! half width short (research/captures/bds-u1fire.txt: 8.0 against the box's 7.875).

use acacia_physics::{Aabb, WorldView};
use sapling_world::World as Level;

use crate::terrain::physics_view;

/// The part of `motion` from `at` gone before the ray meets a collision box, and the axis of the face
/// it met; `None` when it meets none.
pub(super) fn first_block(level: &Level, at: [f32; 3], motion: [f32; 3]) -> Option<(f32, usize)> {
    let swept = Aabb { min: at, max: at }.extend(motion).grow(1e-3);
    let mut boxes = Vec::new();
    physics_view(level, |view| view.collisions(&swept, &mut boxes));
    boxes.iter().filter_map(|block| entered(block, at, motion)).min_by(|a, b| a.0.total_cmp(&b.0))
}

/// The slab test: where in 0..=1 of `motion` the ray from `at` enters `block`, and across which axis.
fn entered(block: &Aabb, at: [f32; 3], motion: [f32; 3]) -> Option<(f32, usize)> {
    let (mut near, mut far, mut axis) = (f32::NEG_INFINITY, f32::INFINITY, None);
    for a in 0..3 {
        if motion[a] == 0.0 {
            if at[a] <= block.min[a] || at[a] >= block.max[a] {
                return None;
            }
            continue;
        }
        let [t0, t1] = [block.min[a], block.max[a]].map(|face| (face - at[a]) / motion[a]);
        let (enter, leave) = (t0.min(t1), t0.max(t1));
        if enter > near {
            (near, axis) = (enter, Some(a));
        }
        far = far.min(leave);
    }
    // A ray that starts inside a box (or on its far side) is not stopped by it.
    (near >= 0.0 && near <= 1.0 && near < far).then_some((near, axis?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ray_enters_by_the_face_it_meets_first() {
        let stone = Aabb { min: [8.0, -26.0, 0.0], max: [9.0, -25.0, 1.0] };
        let (part, axis) = entered(&stone, [6.515, -25.5, 0.5], [2.9627, -0.1613, 0.0]).unwrap();
        assert!((part - 0.501_232).abs() < 1e-4 && axis == 0, "{part} {axis}");
        assert_eq!(entered(&stone, [6.0, -24.0, 0.5], [1.0, 0.0, 0.0]), None, "over it");
        assert_eq!(entered(&stone, [8.5, -25.5, 0.5], [1.0, 0.0, 0.0]), None, "from inside");
        assert_eq!(entered(&stone, [8.5, -24.5, 0.5], [0.0, -1.0, 0.0]), Some((0.5, 1)), "onto its top");
    }
}
