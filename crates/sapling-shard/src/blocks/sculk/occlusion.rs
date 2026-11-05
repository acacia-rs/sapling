//! Wool between a vibration and a listener (docs/research/sculk.md S4): six lines from just off the
//! middle of the source's cell to the middle of the listener's, the vibration lost only when every
//! one passes through wool, as BDS's wool wall stopped one (research/captures/bds-z2sculk.txt).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::name_at;

/// How far off the source cell's middle each line starts.
const OFF: f64 = 0.001;
/// S4's order: up, down, north, south, west, east.
const LINES: [Face; 6] = [Face::Up, Face::Down, Face::North, Face::South, Face::West, Face::East];

fn occludes(name: &str) -> bool {
    name.ends_with("_wool")
}

/// Whether a block put at `pos` is too soft to be heard: wool and carpet (S2.3; bds-z2sculk.txt).
pub(super) fn dampens(world: &World, layer: Entity, pos: BlockPos) -> bool {
    name_at(world, layer, pos).is_some_and(|(name, ..)| occludes(name) || name.ends_with("carpet"))
}

pub(super) fn occluded(world: &World, layer: Entity, from: BlockPos, to: BlockPos) -> bool {
    let (start, end) = (from.map(|c| c as f64 + 0.5), to.map(|c| c as f64 + 0.5));
    LINES.iter().all(|face| {
        let off = face.offset();
        let start = [0, 1, 2].map(|axis| start[axis] + off[axis] as f64 * OFF);
        cells_on(start, end).into_iter().any(|cell| name_at(world, layer, cell).is_some_and(|(name, ..)| occludes(name)))
    })
}

/// The cells a line from `from` to `to` passes through, in order, both ends' included.
fn cells_on(from: [f64; 3], to: [f64; 3]) -> Vec<BlockPos> {
    let mut cell = from.map(|c| c.floor() as i32);
    let last = to.map(|c| c.floor() as i32);
    let delta = [0, 1, 2].map(|axis| to[axis] - from[axis]);
    let step = delta.map(|d| if d > 0.0 { 1 } else if d < 0.0 { -1 } else { 0 });
    let across = delta.map(|d| if d == 0.0 { f64::INFINITY } else { 1.0 / d.abs() });
    let mut next = [0, 1, 2].map(|axis| match step[axis] {
        1 => (cell[axis] as f64 + 1.0 - from[axis]) * across[axis],
        -1 => (from[axis] - cell[axis] as f64) * across[axis],
        _ => f64::INFINITY,
    });
    let mut cells = vec![cell];
    while cell != last {
        let axis = (0..3).min_by(|a, b| next[*a].total_cmp(&next[*b])).unwrap_or(0);
        if next[axis] > 1.0 {
            break;
        }
        cell[axis] += step[axis];
        next[axis] += across[axis];
        cells.push(cell);
    }
    cells
}
