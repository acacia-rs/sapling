//! Where a player rising at its world's spawn stands: a column within `spawnRadius` of it, tried
//! in BDS's order (docs/research/player-damage.md, "Rise spot").

use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

use crate::blocks::Dice;
use crate::components::Layer;

const SPAWN_RADIUS: &str = "spawnRadius";
const DEFAULT_RADIUS: i32 = 10;
/// The inner grid's widest reach; past it, outer cells this far apart are tried.
const INNER_MOST: i32 = 8;
const OUTER_CELL: i32 = 32;
/// Tops a body is not put on.
const UNSAFE: [&str; 4] = ["minecraft:lava", "minecraft:flowing_lava", "minecraft:fire", "minecraft:magma"];

/// The feet a player rising at `spawn` of `layer` is put at, or `spawn` where no column is fit.
pub(super) fn rise_spot(world: &mut World, layer: Entity, spawn: [f32; 3]) -> [f32; 3] {
    let Some(radius) = world.get::<Layer>(layer).map(|layer| match layer.world.level().rules.get(SPAWN_RADIUS) {
        Some((_, sapling_data::RuleValue::Int(radius))) => radius.max(0),
        _ => DEFAULT_RADIUS,
    }) else {
        return spawn;
    };
    let order = order(&mut world.get_resource_or_insert_with(Dice::default), radius);
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return spawn };
    let [cx, cz] = [spawn[0].floor() as i32, spawn[2].floor() as i32];
    order
        .into_iter()
        .find_map(|[dx, dz]| {
            let [x, z] = [cx + dx, cz + dz];
            if !level.is_loaded(ChunkPos::of_block(x, z)) {
                return None;
            }
            let top = level.surface(x, z)?;
            let state = level.registry().get(level.block(x, top, z))?;
            (state.is_solid() && !UNSAFE.contains(&state.name)).then(|| [x as f32 + 0.5, (top + 1) as f32, z as f32 + 0.5])
        })
        .unwrap_or(spawn)
}

/// The offsets from the spawn's column to try, in order: each outer cell's inner grid, both grids
/// walked from a thrown start by a prime stride, kept within the radius's disc.
fn order(dice: &mut Dice, radius: i32) -> Vec<[i32; 2]> {
    let inner = Grid::thrown(dice, radius.min(INNER_MOST));
    let outer_reach = if radius > INNER_MOST && inner.reach > 0 { (radius - INNER_MOST + 2 * inner.reach - 1) / (2 * inner.reach) } else { 0 };
    let outer = Grid::thrown(dice, outer_reach);
    let inner = inner.cells();
    outer
        .cells()
        .into_iter()
        .flat_map(|[ox, oz]| inner.iter().map(move |[x, z]| [ox * OUTER_CELL + x, oz * OUTER_CELL + z]))
        .filter(|[x, z]| x * x + z * z <= radius * radius && radius > 0)
        .collect()
}

/// A square of cells `reach` each way from its centre, visited from `start`.
struct Grid {
    reach: i32,
    start: u32,
}

impl Grid {
    fn thrown(dice: &mut Dice, reach: i32) -> Self {
        let side = (2 * reach + 1) as u32;
        Self { reach, start: dice.roll(side * side) }
    }

    fn cells(&self) -> Vec<[i32; 2]> {
        let side = 2 * self.reach + 1;
        let count = side * side;
        let stride = largest_prime_to(2 * self.reach);
        (0..count)
            .map(|index| (self.start as i32 + index * stride) % count)
            .map(|cell| [cell % side - self.reach, cell / side - self.reach])
            .collect()
    }
}

/// The largest prime no more than `most`, 2 below 3: never a factor of the grid's odd count.
fn largest_prime_to(most: i32) -> i32 {
    (3..=most).rev().find(|n| (2..=n / 2).all(|d| n % d != 0)).unwrap_or(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_order_visits_each_cell_of_the_disc_once() {
        let mut dice = Dice::default();
        let mut cells = order(&mut dice, 10);
        let len = cells.len();
        cells.sort();
        cells.dedup();
        let disc = (-10..=10i32).flat_map(|x| (-10..=10i32).map(move |z| [x, z])).filter(|&[x, z]| x * x + z * z <= 100 && x.abs() <= 8 && z.abs() <= 8).count();
        assert_eq!((len, cells.len()), (disc, disc));
        assert!(order(&mut dice, 0).is_empty());
        assert_eq!(largest_prime_to(16), 13);
    }
}
