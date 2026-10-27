//! Block ticks saved with their column, as BDS saves its `PendingTicks`
//! (`sapling_leveldb::decode_pending_ticks` has the layout): before columns are saved, what waits
//! in the liquid, falling-block, fire and job queues is written into each loaded column, and what
//! waits in a column that is gone is dropped from the queues; a column that comes in has what it
//! kept queued again, as many ticks off as it was.
// What waits elsewhere (a repeater's delay, a pressed button, lit ore) is not kept.

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ChunkPos, PendingTick};

use super::{falling, fire, level, liquid, queued};
use crate::chunks::UNLOAD_INTERVAL;
use crate::components::{Layer, Tick};

fn column_of([x, _, z]: BlockPos) -> ChunkPos {
    ChunkPos::of_block(x, z)
}

/// [`write_back`] on the ticks columns are saved.
pub(crate) fn write_back_before_saving(world: &mut World) {
    if world.resource::<Tick>().0 % UNLOAD_INTERVAL == 0 {
        write_back(world);
    }
}

/// Writes what waits into every loaded column (see the module).
pub(crate) fn write_back(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let mut waiting = liquid::waiting(world);
    waiting.extend(falling::waiting(world));
    waiting.extend(fire::waiting(world));
    waiting.extend(queued::waiting(world));
    waiting.sort_unstable_by_key(|&(_, [x, y, z], due)| (due, x, y, z));
    let mut by_column: HashMap<(Entity, ChunkPos), Vec<(BlockPos, u64)>> = HashMap::new();
    for &(layer, pos, due) in &waiting {
        by_column.entry((layer, column_of(pos))).or_default().push((pos, due));
    }
    let layers: Vec<Entity> = world.query_filtered::<Entity, With<Layer>>().iter(world).collect();
    for layer in layers {
        let Some(level) = level(world, layer) else { continue };
        let records: Vec<(ChunkPos, Vec<u8>)> = level
            .loaded_positions()
            .into_iter()
            .map(|column| {
                let ticks: Vec<([i32; 3], u32, i64)> = by_column.get(&(layer, column)).into_iter().flatten().map(|&(pos, due)| (pos, waiting_state(level, pos), due as i64)).collect();
                (column, sapling_world::encode_pending_ticks(level.registry(), now as i64, &ticks))
            })
            .collect();
        let Some(mut level) = world.get_mut::<Layer>(layer) else { continue };
        for (column, record) in records {
            level.world.set_pending_ticks(column, record);
        }
    }
    let is_gone = |&(layer, column): &(Entity, ChunkPos)| level(world, layer).is_none_or(|level| !level.is_loaded(column));
    let gone: HashSet<(Entity, ChunkPos)> = by_column.into_keys().filter(is_gone).collect();
    if !gone.is_empty() {
        let saved = |layer: Entity, pos: BlockPos| gone.contains(&(layer, column_of(pos)));
        liquid::forget(world, &saved);
        falling::forget(world, &saved);
        fire::forget(world, &saved);
        queued::forget(world, &saved);
    }
}

/// The state a waiting tick is filed under: the liquid a cell holds beside its block, else its block.
fn waiting_state(level: &sapling_world::World, [x, y, z]: BlockPos) -> u32 {
    let held = level.liquid(x, y, z);
    if level.registry().get(held).is_some_and(|state| state.is_liquid()) { held } else { level.block(x, y, z) }
}

/// A column came in: what it kept waiting is queued again by what it is for.
pub(crate) fn arrived(world: &mut World, layer: Entity, column: ChunkPos) {
    let Some(record) = level(world, layer).map(|level| level.pending_ticks(column).to_vec()).filter(|record| !record.is_empty()) else { return };
    let (saved_at, ticks) = match sapling_world::decode_pending_ticks(&record) {
        Ok(read) => read,
        Err(error) => {
            tracing::warn!(x = column.x, z = column.z, %error, "a column's pending ticks are unreadable; dropping them");
            return;
        }
    };
    for PendingTick { name, properties, pos, time } in ticks {
        let delay = (time - saved_at).max(1) as u64;
        if liquid::Liquid::of(&name).is_some() {
            liquid::again(world, layer, pos, delay);
        } else if matches!(name.as_str(), "minecraft:fire" | "minecraft:soul_fire") {
            fire::again(world, layer, pos, delay);
        } else if falling::falls(&name) {
            falling::queue(world, layer, pos, delay);
        } else if let Some(job) = queued::job_of(&name, &properties) {
            queued::again(world, layer, pos, delay, job);
        }
    }
}
