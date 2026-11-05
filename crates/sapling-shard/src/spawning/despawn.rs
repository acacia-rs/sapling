//! Natural mobs going, by their kind's `minecraft:despawn` (`sapling_data::despawn_distances`) as
//! BDS's `DespawnComponent::_canDespawn` (0x141ba30e0) reads it: at once when no player is within
//! its far distance (no more than [`TICK_RANGE_FAR`] where the chunk tick range is 4) or no player's
//! view covers its column (`despawn_from_simulation_edge`), and, idle [`INACTIVE`] ticks with no
//! player within its near distance, by a 1 in [`CHANCE`] roll each tick. Only mobs that appeared by
//! themselves ([`Natural`]) go. Idleness is BDS's `NoActionTimeComponent`: [`count_idle`].

use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

use super::{Natural, SpawnDice, Watcher, watchers};
use crate::body::Body;
use crate::components::InLayer;
use crate::creatures::Health;
use crate::entities::Kind;
use crate::mobs;

/// `despawn_from_chance`'s default one-in.
const CHANCE: u32 = 800;
/// `min_range_inactivity_timer`'s default 30 s.
const INACTIVE: u32 = 600;
/// BDS caps the far distance so where `getChunkTickRange()` is 4 or less, and not past it.
pub(super) const TICK_RANGE_FAR: f32 = 44.0;
pub(super) const CAPPED_TICK_RANGE: i32 = 4;
/// A monster idles 2 ticks more a tick where it is brighter than 0.5 (`MonsterAiStepSystem`): raw
/// brightness 13 by the Java game's light ramp, which BDS's overworld shares [U].
pub(super) const MONSTER_BRIGHT: u8 = 13;
/// Its brightness is read this far up its box (`Actor::getBrightness`, 0x141c652d0).
const BRIGHTNESS_UP: f32 = 0.66;
/// Read as always bright.
const BRIGHT_KINDS: [&str; 2] = ["minecraft:blaze", "minecraft:magma_cube"];
/// Kinds whose `minecraft:despawn` is filters alone, which `_canDespawn` runs instead of its own
/// rules (no view edge, no 44 cap): not persistent and over [`FILTERED_FAR`] from every player, or
/// idle 30 s, the 1 in 800 roll and over [`FILTERED_NEAR`]. The wandering trader's: villagers/wandering.rs.
const FILTERED: [&str; 2] = ["minecraft:piglin_brute", "minecraft:zoglin"];
const FILTERED_NEAR: f32 = 32.0;
const FILTERED_FAR: f32 = 54.0;

/// A tick of a living ticked mob: 1 more tick idle (`PostAIUpdateSystem`), 3 for a bright monster.
pub(crate) fn count_idle(world: &mut World, mob: Entity, layer: Entity, kind: &str) {
    let Some(body) = world.get::<Body>(mob).copied() else { return };
    let bright = || {
        let [x, feet, z] = body.at;
        let at = [x, feet + body.size[1] * BRIGHTNESS_UP, z].map(|c| c.floor() as i32);
        BRIGHT_KINDS.contains(&kind) || crate::light::brightness(world, layer, at).is_some_and(|light| light >= MONSTER_BRIGHT)
    };
    let ticks = if sapling_data::has_family(kind, "monster") && bright() { 3 } else { 1 };
    if let Some(mut health) = world.get_mut::<Health>(mob) {
        health.idle_for(ticks);
    }
}

/// Takes away the natural mobs that are due to go; a player within its near distance ends a mob's idleness.
pub(crate) fn tick(world: &mut World) {
    let watchers = watchers(world);
    let capped = crate::ticking::tick_distance(world) <= CAPPED_TICK_RANGE;
    let natural: Vec<(Entity, Entity, String, [f32; 3], u32)> = world
        .query_filtered::<(Entity, &InLayer, &Kind, &Body, &Health), With<Natural>>()
        .iter(world)
        .map(|(entity, layer, kind, body, health)| (entity, layer.0, kind.0.clone(), body.at, health.idle()))
        .collect();
    let mut gone = Vec::new();
    let mut stirred = Vec::new();
    {
        let mut dice = world.get_resource_or_insert_with(SpawnDice::default);
        for (entity, layer, kind, at, idle) in natural {
            let filtered = FILTERED.contains(&kind.as_str());
            let (near, far) = match sapling_data::despawn_distances(&kind) {
                _ if filtered => (FILTERED_NEAR, FILTERED_FAR),
                Some((near, far)) if capped => (near, far.min(TICK_RANGE_FAR)),
                Some(distances) => distances,
                None => continue,
            };
            match goes(&watchers, layer, at, (near, far, !filtered), || idle >= INACTIVE && dice.0.roll(CHANCE) == 0) {
                Some(true) => gone.push(entity),
                Some(false) if nearest(&watchers, layer, at) <= near => stirred.push(entity),
                _ => {}
            }
        }
    }
    for entity in stirred {
        if let Some(mut health) = world.get_mut::<Health>(entity) {
            health.stir();
        }
    }
    for entity in gone {
        mobs::remove(world, entity);
    }
}

fn nearest(watchers: &[Watcher], layer: Entity, at: [f32; 3]) -> f32 {
    watchers.iter().filter(|watcher| watcher.layer == layer).map(|watcher| watcher.distance(at)).fold(f32::INFINITY, f32::min)
}

/// Whether a mob of `layer` at `at` goes, by its distances and, beyond `near`, `rolled`, and, with
/// `edge`, when no player's view covers it; `None` with no player in its world at all, when BDS
/// despawns nothing and reads nothing.
fn goes(watchers: &[Watcher], layer: Entity, at: [f32; 3], (near, far, edge): (f32, f32, bool), rolled: impl FnOnce() -> bool) -> Option<bool> {
    let column = ChunkPos::of_block(at[0].floor() as i32, at[2].floor() as i32);
    let mut mine = watchers.iter().filter(|watcher| watcher.layer == layer).peekable();
    mine.peek()?;
    if edge && !mine.any(|watcher| watcher.sees(column)) {
        return Some(true);
    }
    let nearest = nearest(watchers, layer, at);
    Some(nearest > far || (nearest > near && rolled()))
}
