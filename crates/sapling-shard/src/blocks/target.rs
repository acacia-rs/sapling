//! Target blocks: a shot that meets one gives a signal the stronger the nearer it hit the middle of
//! the face, for a while (docs/research/redstone-blocks.md, "Target").

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::queued::{self, Job};
use super::{level, redstone};

const TARGET: &str = "minecraft:target";
const FIREBALL: &str = "minecraft:small_fireball";
const GRIEFING: &str = "mobGriefing";
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

/// The cell behind the face across `axis` a shot moving at `motion` met at `at`.
fn met(at: [f32; 3], motion: [f32; 3], axis: usize) -> BlockPos {
    let mut inside = at;
    inside[axis] += motion[axis].signum() * 0.01;
    inside.map(|c| c.floor() as i32)
}

/// A shot of entity type `kind` stopped by a block: a target it met gives its signal, a block it
/// breaks breaks (shot_blocks.rs), a small fireball lights the cell before the face it met (Java
/// `SmallFireball.onHitBlock`; one dispensed at TNT lit no TNT on BDS, research/captures/bds-bkticks.txt),
/// TNT another `burning` one met is lit (`TntBlock.onProjectileHit`). `sticks` for an arrow or trident.
/// A mob's shot lights nothing while `mobGriefing` is off (a blaze's lit none on BDS, bds-u1blaze.txt).
/// `at` is where its ray met the face across `axis` (B25). False where the block lets the shot
/// lie on its face unheld: TNT it lit, primed only next tick (bds-g10flame.txt).
#[allow(clippy::too_many_arguments)]
pub(crate) fn shot_stopped(world: &mut World, layer: Entity, kind: &str, at: [f32; 3], motion: [f32; 3], axis: usize, sticks: bool, burning: bool, by_mob: bool) -> bool {
    let pos = met(at, motion, axis);
    super::shot_blocks::shot(world, layer, pos, kind, motion);
    super::dripleaf::shot(world, layer, pos);
    let lights = !by_mob || crate::rules::on(world, layer, GRIEFING);
    if kind == FIREBALL {
        let mut before = pos;
        before[axis] -= motion[axis].signum() as i32;
        if lights {
            super::fire::light(world, layer, before);
        }
    } else if burning && lights && crate::tnt::light_next_tick(world, layer, pos) {
        return false;
    }
    let is_target = level(world, layer).filter(|level| level.in_height(pos[1])).and_then(|level| level.registry().get(level.block(pos[0], pos[1], pos[2]))).is_some_and(|state| state.name == TARGET);
    if is_target {
        redstone::emit(world, layer, pos, strength(pos, at, axis));
        queued::cancel(world, layer, pos, Job::TargetOff);
        queued::after(world, layer, pos, if sticks { ARROW_TICKS } else { OTHER_TICKS }, Job::TargetOff);
    }
    true
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
        assert_eq!([met([2.5, 4.25, 1.0], [0.0, 0.0, 1.0], 2), met([2.5, 4.0, 1.5], [0.3, -1.0, 0.0], 1)], [[2, 4, 1], [2, 3, 1]]);
    }
}
