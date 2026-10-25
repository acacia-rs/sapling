//! Target blocks: a shot that meets one gives a signal the stronger the nearer it hit the middle of
//! the face, for a while (docs/research/redstone-blocks.md, "Target").

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::queued::{self, Job};
use super::{level, redstone};

const TARGET: &str = "minecraft:target";
/// Ticks the signal lasts after an arrow or trident, and after anything else.
const ARROW_TICKS: u64 = 20;
const OTHER_TICKS: u64 = 8;

/// Half a face's diagonal: a hit this far from the middle gives the least.
const REACH: f32 = 0.707_106_77;

/// The signal a hit at `point` on the face across `axis` of the target at `pos` gives: by its
/// distance from the face's middle (BDS's, docs/research/redstone-blocks.md B26).
fn strength(pos: BlockPos, point: [f32; 3], axis: usize) -> u8 {
    let off = (0..3).filter(|other| *other != axis).map(|other| (point[other] - pos[other] as f32 - 0.5).powi(2)).sum::<f32>().sqrt();
    ((REACH - off) * 16.0 / REACH).clamp(1.0, 15.0) as u8
}

/// The cell and the axis of the face a shot moving at `motion` met, its box (`size`, bottom middle
/// at `at`) stopped short on the axes in `stopped`; and where on that face it hit.
fn met(at: [f32; 3], motion: [f32; 3], size: [f32; 2], stopped: [bool; 3]) -> Option<(BlockPos, usize, [f32; 3])> {
    let axis = (0..3).filter(|axis| stopped[*axis]).max_by(|a, b| motion[*a].abs().total_cmp(&motion[*b].abs()))?;
    let half = [size[0] / 2.0, size[1] / 2.0, size[0] / 2.0];
    let mut point = [at[0], at[1] + half[1], at[2]];
    point[axis] += motion[axis].signum() * half[axis];
    let mut inside = point;
    inside[axis] += motion[axis].signum() * 0.01;
    Some((inside.map(|c| c.floor() as i32), axis, point))
}

/// A shot stopped by a block: a target it met gives its signal. `sticks` for an arrow or trident.
pub(crate) fn shot_stopped(world: &mut World, layer: Entity, at: [f32; 3], motion: [f32; 3], size: [f32; 2], stopped: [bool; 3], sticks: bool) {
    let Some((pos, axis, point)) = met(at, motion, size, stopped) else { return };
    let is_target = level(world, layer).filter(|level| level.in_height(pos[1])).and_then(|level| level.registry().get(level.block(pos[0], pos[1], pos[2]))).is_some_and(|state| state.name == TARGET);
    if !is_target {
        return;
    }
    redstone::emit(world, layer, pos, strength(pos, point, axis));
    queued::cancel(world, layer, pos, Job::TargetOff);
    queued::after(world, layer, pos, if sticks { ARROW_TICKS } else { OTHER_TICKS }, Job::TargetOff);
}

pub(super) fn ended(world: &mut World, layer: Entity, pos: BlockPos) {
    redstone::emit(world, layer, pos, 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearer_the_middle_the_stronger() {
        let on = |x: f32, y: f32| strength([0, 3, 7], [x, y, 7.0], 2);
        assert_eq!([on(0.5, 3.5), on(0.95, 3.5), on(0.75, 3.5), on(0.999, 3.001)], [15, 5, 10, 1]);
        assert_eq!(met([2.5, 4.0, 0.75], [0.0, 0.0, 1.0], [0.5, 0.5], [false, false, true]), Some(([2, 4, 1], 2, [2.5, 4.25, 1.0])));
    }
}
