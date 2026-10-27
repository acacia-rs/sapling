//! A blast's rays: which blocks it breaks, and how much of a body it sees (BDS's
//! `Explosion::explode` and `BlockSource::getSeenPercent`, docs/research/block-uses.md "Blasts").

use acacia_physics::{Aabb, WorldView};
use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use crate::blocks::Dice;
use crate::components::Layer;
use crate::terrain::physics_view;

/// From the middle to each cell of the faces of a cube this many across, in steps this long,
/// each step costing this much and `(resistance + STEP) x STEP` of the block it is in.
const RAYS: i32 = 16;
const STEP: f32 = 0.3;
const FADES: f32 = 0.225;
/// A body's points are looked at from this far above its bottom.
const RAISED: f32 = 0.1;

fn is_water(level: &Level, id: u32) -> bool {
    level.registry().get(id).is_some_and(|state| matches!(state.name, "minecraft:water" | "minecraft:flowing_water"))
}

/// Whether there is water in the cell, as a block or in its liquid layer.
fn wet(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    level.in_height(y) && (is_water(level, level.block(x, y, z)) || is_water(level, level.liquid(x, y, z)))
}

/// The cells a blast of `power` at `at` breaks, none where its middle is in water. A ray stops
/// at water; air costs it nothing.
pub(super) fn broken(world: &mut World, layer: Entity, at: [f32; 3], power: f32) -> Vec<[i32; 3]> {
    let strengths: Vec<f32> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (0..RAYS * RAYS * RAYS).map(|_| power * (0.7 + dice.fraction() * 0.6)).collect()
    };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return Vec::new() };
    if wet(level, at.map(|c| c.floor() as i32)) {
        return Vec::new();
    }
    let mut cells = Vec::new();
    let edge = |i: i32| i == 0 || i == RAYS - 1;
    for (ray, [i, j, k]) in (0..RAYS).flat_map(|i| (0..RAYS).flat_map(move |j| (0..RAYS).map(move |k| [i, j, k]))).enumerate().filter(|(_, [i, j, k])| edge(*i) || edge(*j) || edge(*k)) {
        let towards = [i, j, k].map(|c| c as f32 / (RAYS - 1) as f32 * 2.0 - 1.0);
        let long = towards.iter().map(|c| c * c).sum::<f32>().sqrt();
        let (mut left, mut here) = (strengths[ray], at);
        while left > 0.0 {
            let cell = here.map(|c| c.floor() as i32);
            if wet(level, cell) {
                break;
            }
            let [x, y, z] = cell;
            let state = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten();
            if let Some(state) = state.filter(|state| !state.is_air()) {
                left -= (sapling_data::blast_resistance(state.name).unwrap_or(0.0) + STEP) * STEP;
                if left > 0.0 && !cells.contains(&cell) {
                    cells.push(cell);
                }
            }
            here = std::array::from_fn(|axis| here[axis] + towards[axis] / long * STEP);
            left -= FADES;
        }
    }
    cells
}

/// The share of points through a body's box `min` to `max` that see `at` past every block's
/// collision boxes: all of it when `at` is inside.
pub(super) fn seen(level: &Level, at: [f32; 3], min: [f32; 3], max: [f32; 3]) -> f32 {
    if (0..3).all(|axis| min[axis] <= at[axis] && at[axis] <= max[axis]) {
        return 1.0;
    }
    let steps: [f32; 3] = std::array::from_fn(|axis| 1.0 / ((max[axis] - min[axis]) * 2.0 + 1.0));
    // Across, the points are centred in what the steps leave over; up, they start a little above the bottom.
    let inset = |axis: usize| (1.0 - (1.0 / steps[axis]).floor() * steps[axis]) * 0.5;
    physics_view(level, |view| {
        let (mut sees, mut all) = (0u32, 0u32);
        let mut fx = 0.0;
        while fx <= 1.0 {
            let mut fy = 0.0;
            while fy <= 1.0 {
                let mut fz = 0.0;
                while fz <= 1.0 {
                    let from = [
                        (max[0] - min[0]) * fx + min[0] + inset(0),
                        (max[1] - min[1]) * fy + min[1] + RAISED,
                        (max[2] - min[2]) * fz + min[2] + inset(2),
                    ];
                    sees += u32::from(!blocked(view, from, at));
                    all += 1;
                    fz += steps[2];
                }
                fy += steps[1];
            }
            fx += steps[0];
        }
        sees as f32 / all as f32
    })
}

/// Whether a block's collision box is on the line from `from` to `to`.
fn blocked(view: &impl WorldView, from: [f32; 3], to: [f32; 3]) -> bool {
    let way: [f32; 3] = std::array::from_fn(|axis| to[axis] - from[axis]);
    let mut boxes = Vec::new();
    cells_on(from, to).into_iter().any(|cell| {
        boxes.clear();
        view.block_collisions(cell, &mut boxes);
        boxes.iter().any(|local| crosses(local.translate(cell.map(|c| c as f32)), from, way))
    })
}

/// The cells the line from `from` to `to` passes through, in order.
fn cells_on(from: [f32; 3], to: [f32; 3]) -> Vec<[i32; 3]> {
    let (mut cell, end) = (from.map(|c| c.floor() as i32), to.map(|c| c.floor() as i32));
    let way: [f32; 3] = std::array::from_fn(|axis| to[axis] - from[axis]);
    let step: [i32; 3] = way.map(|d| if d > 0.0 { 1 } else if d < 0.0 { -1 } else { 0 });
    let span: [f32; 3] = std::array::from_fn(|axis| if step[axis] == 0 { f32::INFINITY } else { 1.0 / way[axis].abs() });
    let mut next: [f32; 3] = std::array::from_fn(|axis| match step[axis] {
        0 => f32::INFINITY,
        1 => ((cell[axis] + 1) as f32 - from[axis]) * span[axis],
        _ => (from[axis] - cell[axis] as f32) * span[axis],
    });
    let mut cells = vec![cell];
    while cell != end && cells.len() < 256 {
        let axis = (0..3).min_by(|a, b| next[*a].total_cmp(&next[*b])).expect("three axes");
        if next[axis] > 1.0 {
            break;
        }
        cell[axis] += step[axis];
        next[axis] += span[axis];
        cells.push(cell);
    }
    cells
}

/// Whether the segment from `from` along `way` meets the box.
fn crosses(bounds: Aabb, from: [f32; 3], way: [f32; 3]) -> bool {
    let (mut enter, mut leave) = (0.0f32, 1.0f32);
    for axis in 0..3 {
        let (low, high) = (bounds.min[axis], bounds.max[axis]);
        if way[axis].abs() < 1e-9 {
            if from[axis] < low || from[axis] > high {
                return false;
            }
            continue;
        }
        let [a, b] = [(low - from[axis]) / way[axis], (high - from[axis]) / way[axis]];
        enter = enter.max(a.min(b));
        leave = leave.min(a.max(b));
        if enter > leave {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_walks_every_cell_it_passes_through() {
        assert_eq!(cells_on([0.5, 0.5, 0.5], [2.5, 0.5, 0.5]), [[0, 0, 0], [1, 0, 0], [2, 0, 0]]);
        assert_eq!(cells_on([0.5, 0.5, 0.5], [-0.5, 1.5, 0.5]).len(), 3);
    }

    /// bds-blasts.txt: TNT on a floor, a pig 3 off in the open, and pigs as far behind a wall
    /// one high and one three high, which only the hit's own throw moved.
    #[test]
    fn a_pig_in_the_open_sees_a_blast_and_one_behind_a_wall_one_high_sees_none_of_it() {
        let mut level = Level::void();
        let stone = level.block_id("minecraft:stone").unwrap();
        level.fill([-8, 63, -8], [8, 63, 8], stone);
        level.fill([-1, 64, -2], [1, 64, -2], stone);
        let at = [0.5, 64.06125, 0.5];
        let pig = |x: f32, z: f32| seen(&level, at, [x - 0.45, 64.0, z - 0.45], [x + 0.45, 64.9, z + 0.45]);
        assert_eq!(pig(3.5, 0.5), 1.0);
        assert_eq!(pig(0.5, -2.5), 0.0);
        let half_hidden = seen(&level, at, [3.05, 64.0, -2.95], [3.95, 64.9, -2.05]);
        assert!(half_hidden > 0.0 && half_hidden < 1.0, "past the wall's end: {half_hidden}");
    }

    #[test]
    fn a_segment_meets_a_box_it_passes_through_and_not_one_it_stops_short_of() {
        let unit = Aabb::new(1.0, 0.0, 0.0, 2.0, 1.0, 1.0);
        assert!(crosses(unit, [0.5, 0.5, 0.5], [2.0, 0.0, 0.0]));
        assert!(!crosses(unit, [0.5, 0.5, 0.5], [0.4, 0.0, 0.0]));
        assert!(!crosses(unit, [0.5, 1.5, 0.5], [2.0, 0.0, 0.0]));
    }
}
