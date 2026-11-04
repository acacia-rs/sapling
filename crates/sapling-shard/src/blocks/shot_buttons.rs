//! Wooden buttons with an arrow or trident in them: BDS pressed one the tick after an arrow came to
//! rest on it, kept it in while the arrow stayed and let it out at the first 30-tick check after the
//! arrow went; a stone button stayed out (research/captures/bds-rsparts.txt, docs/research/redstone.md C13).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{doors, level};

const SHOTS: [&str; 2] = ["minecraft:arrow", "minecraft:thrown_trident"];
const FACING: &str = "facing_direction";
/// A button's face, across, in sixteenths of its cell; and its depth out and pressed in.
const ACROSS: [f32; 2] = [5.0 / 16.0, 11.0 / 16.0];
const DEPTH: [f32; 2] = [2.0 / 16.0, 1.0 / 16.0];

/// A box as its least and greatest corner.
type Box3 = ([f32; 3], [f32; 3]);

/// The box of a wooden button at `pos`, when there is one.
fn wooden_button(world: &World, layer: Entity, pos: BlockPos) -> Option<Box3> {
    let level = level(world, layer).filter(|level| level.in_height(pos[1]))?;
    let state = level.registry().get(level.block(pos[0], pos[1], pos[2]))?;
    if !state.name.ends_with("button") || doors::is_stone_button(state.name) {
        return None;
    }
    let get = |key: &str| state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).map(|(_, value)| value);
    let facing: usize = get(FACING)?.parse().ok()?;
    let depth = DEPTH[usize::from(get("button_pressed_bit") == Some("1"))];
    // `facing_direction` 0..6: hung under a block, on top of one, then north, south, west, east of one.
    let (axis, from_far_side) = [(1, true), (1, false), (2, true), (2, false), (0, true), (0, false)].get(facing).copied()?;
    let mut min = [ACROSS[0]; 3];
    let mut max = [ACROSS[1]; 3];
    (min[axis], max[axis]) = if from_far_side { (1.0 - depth, 1.0) } else { (0.0, depth) };
    Some((std::array::from_fn(|i| pos[i] as f32 + min[i]), std::array::from_fn(|i| pos[i] as f32 + max[i])))
}

fn overlaps((min, max): Box3, (other_min, other_max): Box3) -> bool {
    (0..3).all(|axis| min[axis] < other_max[axis] && max[axis] > other_min[axis])
}

/// The boxes of the arrows and tridents in `layer`.
fn shots(world: &World, layer: Entity) -> Vec<Box3> {
    crate::entities::bodies(world).into_iter().filter(|(at, kind, _)| *at == layer && SHOTS.contains(&kind.as_str())).map(|(_, _, body)| body.bounds()).collect()
}

/// Whether an arrow or trident is in the wooden button at `pos`.
pub(super) fn holds(world: &World, layer: Entity, pos: BlockPos) -> bool {
    wooden_button(world, layer, pos).is_some_and(|button| shots(world, layer).into_iter().any(|shot| overlaps(button, shot)))
}

/// Presses every wooden button an arrow or trident is in, as it was at the end of the last tick.
pub(crate) fn press_shot_buttons(world: &mut World) {
    let shot: Vec<(Entity, BlockPos)> = crate::entities::bodies(world)
        .into_iter()
        .filter(|(_, kind, _)| SHOTS.contains(&kind.as_str()))
        .flat_map(|(layer, _, body)| {
            let (min, max) = body.bounds();
            let span = |axis: usize| min[axis].floor() as i32..=max[axis].floor() as i32;
            span(0).flat_map(move |x| span(1).flat_map(move |y| span(2).map(move |z| (layer, [x, y, z])))).collect::<Vec<_>>()
        })
        .collect();
    for (layer, pos) in shot {
        if holds(world, layer, pos) {
            doors::press(world, layer, pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shot_lying_on_the_floor_is_in_a_floor_button_but_not_one_on_a_wall_above() {
        let floor = ([5.0 / 16.0, 0.0, 5.0 / 16.0], [11.0 / 16.0, 2.0 / 16.0, 11.0 / 16.0]);
        let arrow = ([0.25, 0.0, 0.25], [0.75, 0.5, 0.75]);
        assert!(overlaps(floor, arrow));
        assert!(!overlaps(([0.0, 0.6, 0.3], [0.1, 0.7, 0.7]), arrow));
    }
}
