//! A mob's step up: one stopped across while it stood on the ground, or as it lands, tries the move again
//! from up to its `max_auto_step` higher and back down, and takes that where it gets further across
//! (BDS's actor move as acacia-physics `auto_step` has the player's; research/captures/bds-bc1step.txt).

use acacia_physics::{Aabb, WorldView, collide_box};

use super::{Body, overlapped};

/// Where a box at `start` moving by `motion` gets stepping up to `height`: `None` where that ends in a
/// block.
fn stepped(view: &impl WorldView, start: &Aabb, motion: [f32; 3], height: f32) -> Option<[f32; 3]> {
    let up = collide_box(view, start, [0.0, height, 0.0]);
    let raised = start.translate(up);
    let across = collide_box(view, &raised, [motion[0], 0.0, motion[2]]);
    let moved = raised.translate(across);
    let down = collide_box(view, &moved, [0.0, -up[1], 0.0]);
    let end = moved.translate(down);
    overlapped(view, &end).is_empty().then_some([across[0], up[1] + down[1], across[2]])
}

fn across(moved: [f32; 3]) -> f32 {
    moved[0] * moved[0] + moved[2] * moved[2]
}

impl Body {
    /// How far a mob that `stood` on the ground gets moving by `motion`, stepping up to `height`.
    pub(super) fn step_move(&self, view: &impl WorldView, motion: [f32; 3], height: f32, stood: bool) -> [f32; 3] {
        let start = self.aabb();
        let moved = collide_box(view, &start, motion);
        let stopped = moved[0] != motion[0] || moved[2] != motion[2];
        let lands = motion[1] < 0.0 && moved[1] != motion[1];
        if height <= 0.0 || !stopped || !(stood || lands) {
            return moved;
        }
        stepped(view, &start, motion, height).filter(|step| across(*step) > across(moved)).unwrap_or(moved)
    }
}
