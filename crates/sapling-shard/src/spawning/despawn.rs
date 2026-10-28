//! Natural mobs going, by their kind's `minecraft:despawn` (`sapling_data::despawn_distances`) as
//! BDS's `DespawnComponent::_canDespawn` reads it at tick range 4: at once when no player is within
//! its far distance (44 at most) or no player's view covers its column (`despawn_from_simulation_edge`),
//! and, inactive [`INACTIVE`] ticks with no player within its near distance, by a 1 in [`CHANCE`]
//! roll each tick. Only mobs that appeared by themselves ([`Natural`]) go.

use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

use super::{Natural, SpawnDice, Watcher, watchers};
use crate::body::Body;
use crate::components::{InLayer, Tick};
use crate::entities::Kind;
use crate::mobs;

/// `despawn_from_chance`'s default one-in.
const CHANCE: u32 = 800;
/// `min_range_inactivity_timer`'s default 30 s.
// TODO: BDS counts ticks since the mob was last active; here since it appeared.
const INACTIVE: u64 = 600;
/// Where the chunk tick range is 4, the far distance is no more than this.
// TODO: BDS's cap at other `tick-distance`s is unmeasured; this one is kept at every distance.
const TICK_RANGE_FAR: f32 = 44.0;

/// Takes away the natural mobs that are due to go.
pub(crate) fn tick(world: &mut World) {
    let watchers = watchers(world);
    let now = world.resource::<Tick>().0;
    let natural: Vec<(Entity, Entity, String, [f32; 3], u64)> = world
        .query::<(Entity, &InLayer, &Kind, &Body, &Natural)>()
        .iter(world)
        .map(|(entity, layer, kind, body, natural)| (entity, layer.0, kind.0.clone(), body.at, now.saturating_sub(natural.born)))
        .collect();
    let mut gone = Vec::new();
    {
        let mut dice = world.get_resource_or_insert_with(SpawnDice::default);
        for (entity, layer, kind, at, age) in natural {
            let Some((near, far)) = sapling_data::despawn_distances(&kind) else { continue };
            if goes(&watchers, layer, at, near, far.min(TICK_RANGE_FAR), || age >= INACTIVE && dice.0.roll(CHANCE) == 0) {
                gone.push(entity);
            }
        }
    }
    for entity in gone {
        mobs::remove(world, entity);
    }
}

/// Whether a mob of `layer` at `at` goes, by its distances and, beyond `near`, `rolled`.
fn goes(watchers: &[Watcher], layer: Entity, at: [f32; 3], near: f32, far: f32, rolled: impl FnOnce() -> bool) -> bool {
    let column = ChunkPos::of_block(at[0].floor() as i32, at[2].floor() as i32);
    let mine = || watchers.iter().filter(|watcher| watcher.layer == layer);
    // With no player at all, BDS despawns nothing.
    if mine().next().is_none() {
        return false;
    }
    if !mine().any(|watcher| watcher.sees(column)) {
        return true;
    }
    let nearest = mine().map(|watcher| watcher.distance(at)).fold(f32::INFINITY, f32::min);
    nearest > far || (nearest > near && rolled())
}
