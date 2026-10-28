//! What mobs do with turtle eggs besides treading on them: a turtle lays them on sand, a zombie
//! goes for them and stomps them out (creatures/nesting.rs, creatures/stomping.rs).

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos};

use super::turtle::{COUNTS, EGG, SANDS, break_one, is_egg_at};
use super::{level, name_at, send};

const AIR: &str = "minecraft:air";

/// The cells within `across` and `up` of `at`, nearest first, that `fits`.
fn nearest(world: &World, layer: Entity, at: [f32; 3], across: i32, up: i32, fits: fn(&super::Level, BlockPos) -> bool) -> Option<BlockPos> {
    let level = level(world, layer)?;
    let [x, y, z] = at.map(|c| c.floor() as i32);
    let mut cells: Vec<BlockPos> = (-across..=across).flat_map(|dx| (-up..=up).flat_map(move |dy| (-across..=across).map(move |dz| [x + dx, y + dy, z + dz]))).collect();
    let away = |[cx, cy, cz]: &BlockPos| (cx - x).pow(2) + (cy - y).pow(2) + (cz - z).pow(2);
    cells.sort_by_key(away);
    cells.into_iter().find(|cell| fits(level, *cell))
}

/// The nearest turtle egg within `across` and `up` of `at`.
pub(crate) fn egg_near(world: &World, layer: Entity, at: [f32; 3], across: i32, up: i32) -> Option<BlockPos> {
    nearest(world, layer, at, across, up, |level, cell| name_at(level, cell) == Some(EGG))
}

fn is_nest(level: &super::Level, [x, y, z]: BlockPos) -> bool {
    name_at(level, [x, y, z]) == Some(AIR) && name_at(level, [x, y - 1, z]).is_some_and(|under| SANDS.contains(&under))
}

/// The nearest open cell on sand within `across` and `up` of `at`: where a turtle lays.
pub(crate) fn nest_near(world: &World, layer: Entity, at: [f32; 3], across: i32, up: i32) -> Option<BlockPos> {
    nearest(world, layer, at, across, up, is_nest)
}

/// Lays `count` (1 to 4) fresh turtle eggs at `pos`, still open on sand; false where it cannot.
pub(crate) fn lay(world: &mut World, layer: Entity, pos: BlockPos, count: usize) -> bool {
    let Some(eggs) = COUNTS.get(count.wrapping_sub(1)).filter(|_| level(world, layer).is_some_and(|level| is_nest(level, pos))) else { return false };
    let block = Block::new(EGG).with(format!("cracked_state=no_cracks,turtle_egg_count={eggs}"));
    let Some(id) = level(world, layer).and_then(|level| send::resolve(level.registry(), &block)) else { return false };
    send::set(world, layer, pos, id)
}

/// Every egg at `pos` breaks, one after another, the block with the last.
pub(crate) fn stomp_out(world: &mut World, layer: Entity, pos: BlockPos) {
    for _ in 0..COUNTS.len() {
        if !is_egg_at(world, layer, pos) {
            return;
        }
        break_one(world, layer, pos);
    }
}
