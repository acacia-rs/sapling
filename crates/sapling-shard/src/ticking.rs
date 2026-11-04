//! Which furnaces, hoppers, campfires, lit beacons and spawner blocks (spawners.rs, every one) tick. BDS ticks block actors only near players; here one
//! ticks while its column is in a player's view. Every [`UNLOAD_INTERVAL`] ticks, before columns
//! are saved, those in seen columns that have work start (a furnace that was burning when its
//! world was saved, any hopper, a campfire with food on it), and those in columns nobody sees any
//! more stop, their state kept in their block entities. A placed hopper, an opened furnace and a
//! campfire given food start at once.

use std::collections::{HashMap, HashSet};

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ChunkPos;

use crate::chunks::UNLOAD_INTERVAL;
use crate::components::{InLayer, Layer, Tick};
use crate::view::View;
use crate::{beacons, brewing, campfires, furnaces, hoppers, spawners};

fn id(entity: &Value) -> Option<&str> {
    match entity.get("id") {
        Some(Value::String(id)) => Some(id),
        _ => None,
    }
}

/// The chunks about a player that are simulated (`ShardConfig::tick_distance`).
pub(crate) fn tick_distance(world: &World) -> i32 {
    world.resource::<crate::components::Settings>().0.tick_distance
}

/// Whether the column `dx`, `dz` chunks off a player's is one BDS ticks about it at `tick-distance`
/// `reach`: `ChunkTickOffsetManager::initialize` (0x1409e6bd0) lists the rows |z| = 1..=reach to
/// |x| <= reach + 1 - |z| and the row z = 0 to |x| <= reach (docs/research/mob-damage.md).
pub(crate) fn in_tick_region(dx: i32, dz: i32, reach: i32) -> bool {
    dx.abs() <= reach && dz.abs() <= reach && dx.abs() + dz.abs() <= reach + 1
}

/// The columns that tick this tick: each player's tick region ([`in_tick_region`]) and the kept
/// columns (kept.rs), taken once a tick.
pub(crate) struct Region {
    centres: HashMap<Entity, Vec<ChunkPos>>,
    reach: i32,
}

impl Region {
    pub(crate) fn of(world: &mut World) -> Self {
        let mut centres: HashMap<Entity, Vec<ChunkPos>> = HashMap::new();
        for (in_layer, view) in world.query::<(&InLayer, &View)>().iter(world) {
            centres.entry(in_layer.0).or_default().extend(view.center());
        }
        Self { centres, reach: tick_distance(world) }
    }

    /// Whether `column` of `layer` ticks. A world nobody is in ticks everything: BDS's ticks
    /// nothing, but headless shards and their tests run without players.
    pub(crate) fn ticks(&self, world: &World, layer: Entity, column: ChunkPos) -> bool {
        let Some(centres) = self.centres.get(&layer) else { return true };
        centres.iter().any(|centre| in_tick_region(column.x - centre.x, column.z - centre.z, self.reach)) || world.get::<crate::kept::Kept>(layer).is_some_and(|kept| kept.holds(column))
    }

    pub(crate) fn ticks_block(&self, world: &World, layer: Entity, pos: BlockPos) -> bool {
        self.ticks(world, layer, ChunkPos::of_block(pos[0], pos[2]))
    }

    /// Whether a block tick due `when` at `pos` runs at `now`: a BDS chunk's block queue runs only while
    /// the chunk ticks, and then up to the level's tick (`LevelChunk::tick` 0x1409f98a0), so a tick that
    /// fell due out of range waits, its time kept, and runs as its column ticks again.
    pub(crate) fn runs_due(&self, world: &World, when: u64, now: u64, layer: Entity, pos: BlockPos) -> bool {
        when <= now && self.ticks_block(world, layer, pos)
    }

    /// Puts back by `by(layer)` the time of each `(when, layer, pos)` of `queue` whose column does not
    /// tick: a BDS chunk's random-tick queue runs on only while its chunk ticks (`LevelChunk::tick`
    /// 0x1409f98a0 takes its `until` as the queue's own clock plus `randomTickSpeed`).
    pub(crate) fn hold(&self, world: &World, queue: &mut [(u64, Entity, BlockPos)], by: impl Fn(Entity) -> u64) {
        for (when, layer, pos) in queue {
            if !self.ticks_block(world, *layer, *pos) {
                *when += by(*layer);
            }
        }
    }
}

/// Whether `pos` of `layer` ticks ([`Region::ticks`]).
pub(crate) fn ticks_column(world: &mut World, layer: Entity, pos: ChunkPos) -> bool {
    Region::of(world).ticks(world, layer, pos)
}

/// The columns of [`near_players`] and the kept ones (kept.rs), each once, in an order that does
/// not turn on the hash.
pub(crate) fn ticked_columns(world: &mut World) -> Vec<(Entity, ChunkPos)> {
    let mut columns = near_players(world);
    for (layer, kept) in world.query::<(Entity, &crate::kept::Kept)>().iter(world) {
        columns.extend(kept.columns().map(|pos| (layer, pos)));
    }
    columns.sort_by_key(|(layer, pos)| (*layer, pos.x, pos.z));
    columns.dedup();
    columns
}

/// The columns of each player's tick region ([`in_tick_region`]), each once, in an order that
/// does not turn on the hash.
fn near_players(world: &mut World) -> Vec<(Entity, ChunkPos)> {
    let reach = tick_distance(world);
    let mut near = Vec::new();
    for (in_layer, view) in world.query::<(&InLayer, &View)>().iter(world) {
        let Some(center) = view.center() else { continue };
        let offsets = (-reach..=reach).flat_map(|dx| (-reach..=reach).map(move |dz| (dx, dz))).filter(|&(dx, dz)| in_tick_region(dx, dz, reach));
        near.extend(offsets.map(|(dx, dz)| (in_layer.0, ChunkPos::new(center.x + dx, center.z + dz))));
    }
    near.sort_by_key(|(layer, pos)| (*layer, pos.x, pos.z));
    near.dedup();
    near
}

/// The loaded columns some player's view covers or the world keeps (kept.rs), by layer.
fn seen(world: &mut World) -> HashSet<(Entity, ChunkPos)> {
    let (mut players, mut layers) = (world.query::<(&InLayer, &View)>(), world.query::<(Entity, &Layer, Option<&crate::kept::Kept>)>());
    let world = &*world;
    let mut seen = HashSet::new();
    for (layer, level, kept) in layers.iter(world) {
        let is_seen = |pos: &ChunkPos| kept.is_some_and(|kept| kept.holds(*pos)) || players.iter(world).any(|(in_layer, view)| in_layer.0 == layer && view.covers(*pos));
        seen.extend(level.world.loaded().filter(is_seen).map(|pos| (layer, pos)));
    }
    seen
}

/// Starts and stops what ticks by who sees its column; see the module docs.
pub(crate) fn sweep(world: &mut World) {
    if world.resource::<Tick>().0 % UNLOAD_INTERVAL != 0 {
        return;
    }
    let seen = seen(world);
    let column = |pos: BlockPos| ChunkPos::of_block(pos[0], pos[2]);
    let watched: Vec<(Entity, BlockPos)> =
        [furnaces::watched(world), brewing::watched(world), hoppers::watched(world), campfires::watched(world), spawners::watched(world), beacons::watched(world)].concat();
    for (layer, pos) in watched.into_iter().filter(|(layer, pos)| !seen.contains(&(*layer, column(*pos)))) {
        beacons::forget(world, layer, pos);
        furnaces::forget(world, layer, pos);
        brewing::forget(world, layer, pos);
        hoppers::forget(world, layer, pos);
        campfires::forget(world, layer, pos);
        spawners::forget(world, layer, pos);
    }
    type Watch = fn(&mut World, Entity, BlockPos);
    let mut found: Vec<(Entity, BlockPos, Watch)> = Vec::new();
    for &(layer, pos) in &seen {
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { continue };
        for entity in level.block_entities_in(pos) {
            let watch: Watch = match id(&entity.nbt) {
                Some("Hopper") => hoppers::watch,
                Some("Beacon") if beacons::is_lit(&entity.nbt) => beacons::watch,
                Some("Furnace" | "BlastFurnace" | "Smoker") if furnaces::was_at_work(&entity.nbt) => furnaces::watch,
                Some("BrewingStand") if brewing::was_at_work(&entity.nbt) => brewing::watch,
                Some("Campfire") if campfires::is_cooking(&entity.nbt) => campfires::watch,
                _ if spawners::is_spawner(&entity.nbt) => spawners::watch,
                _ => continue,
            };
            found.push((layer, entity.pos, watch));
        }
    }
    for (layer, pos, watch) in found {
        watch(world, layer, pos);
    }
}

#[cfg(test)]
mod tests {
    use super::in_tick_region;

    /// BDS's offset list as 0x1409e6bd0 builds it.
    fn bds_offsets(reach: i32) -> Vec<(i32, i32)> {
        let mut offsets = Vec::new();
        for i in 0..reach {
            for x in -(i + 1)..=(i + 1) {
                offsets.extend([(x, reach - i), (x, i - reach)]);
                if i == reach - 1 {
                    offsets.push((x, 0));
                }
            }
        }
        offsets
    }

    #[test]
    fn the_tick_region_is_bds_offset_list() {
        for reach in 4..=12 {
            let mut listed = bds_offsets(reach);
            listed.sort_unstable();
            let region: Vec<_> = (-reach - 2..=reach + 2).flat_map(|x| (-reach - 2..=reach + 2).map(move |z| (x, z))).filter(|&(x, z)| in_tick_region(x, z, reach)).collect();
            assert_eq!(region, listed, "{reach}");
        }
        // research/captures/bds-tickcircle6.txt and 10: 5,3 off out at 6, 7,4 and 6,5 in at 10, 6,6 out.
        assert!(in_tick_region(3, 3, 6) && in_tick_region(6, 0, 6) && !in_tick_region(5, 3, 6) && !in_tick_region(7, 0, 6));
        assert!(in_tick_region(7, 4, 10) && in_tick_region(6, 5, 10) && !in_tick_region(6, 6, 10) && !in_tick_region(10, 2, 10));
    }
}
