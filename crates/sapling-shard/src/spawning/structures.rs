//! Structure spawning, BDS's pass after a spawn attempt's column (`BedrockSpawner::tick` 1415eebf0):
//! the ticked chunk's spawn-area pieces of a structure with its own list (fortress, monument, swamp
//! hut, outpost) give their middles; a few tries each pick one, find a spot down from it and put a
//! herd of an entry of the list there. A natural cluster whose floor is in such a piece is skipped.
//! The areas are the column's saved ones (sapling_world::SpawnArea). See docs/research/structure-spawns.md.

mod lists;

use bevy_ecs::prelude::*;
use sapling_data::Medium;
use sapling_world::{ChunkPos, SpawnArea};

use super::despawn::{CAPPED_TICK_RANGE, TICK_RANGE_FAR};
use super::herds::{ALL, CAPS, count, difficulty, fraction, herd_size, spawn};
use super::spots::{self, Found};
use super::Watcher;
use crate::components::Layer;
use lists::{Entry, list};

pub(super) use lists::category;

/// `isSpawnPositionOk`'s distances (14118c440): the spawn rules' defaults, the far one capped as despawning's.
const NEAREST: f32 = 24.0;
const FARTHEST: f32 = 128.0;

fn contains(area: &SpawnArea, [x, y, z]: [i32; 3]) -> bool {
    let [x0, y0, z0, x1, y1, z1] = area.bounds;
    (x0..=x1).contains(&x) && (y0..=y1).contains(&y) && (z0..=z1).contains(&z)
}

/// The list of the spawning structure whose piece holds `block`; only pieces count, as all four
/// structures register for them alone.
fn list_at(world: &World, layer: Entity, block: [i32; 3]) -> Option<&'static [Entry]> {
    let level = &world.get::<Layer>(layer)?.world;
    let areas = level.spawn_areas(ChunkPos::of_block(block[0], block[2]));
    areas.iter().filter(|area| !area.whole && contains(area, block)).find_map(|area| list(&area.structure))
}

fn floor(found: &Found) -> [i32; 3] {
    let [x, y, z] = found.cell;
    [x, y - 1, z]
}

/// Whether a natural cluster at `found` is left to its structure.
pub(super) fn in_piece(world: &World, layer: Entity, found: &Found) -> bool {
    list_at(world, layer, floor(found)).is_some()
}

/// A piece's middle as the pass starts from: its top lowered by the structure's offset.
fn middle(area: &SpawnArea) -> [i32; 3] {
    let [x0, _, z0, x1, y1, z1] = area.bounds;
    [x0 + ((x1 - x0 + 1) >> 1), y1 + area.y_offset.unwrap_or(0), z0 + ((z1 - z0 + 1) >> 1)]
}

fn roll(world: &mut World, n: usize) -> usize {
    ((fraction(world) * n as f32) as usize).min(n.saturating_sub(1))
}

/// The structure pass of an attempt in `chunk`; `top` is the medium of the attempt column's first spot.
pub(super) fn attempt(world: &mut World, layer: Entity, chunk: ChunkPos, top: Medium, watchers: &[Watcher]) {
    // Every list is of monsters, which peaceful spawns none of.
    if difficulty(world) == 0 {
        return;
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let middles: Vec<[i32; 3]> = level.spawn_areas(chunk).iter().filter(|area| !area.whole && list(&area.structure).is_some()).map(middle).collect();
    if middles.is_empty() {
        return;
    }
    for _ in 0..middles.len().max(2) / 2 {
        let [x, y, z] = middles[roll(world, middles.len())];
        // Drilled: monument guardians stood at the middle's own y (61), outpost pillagers on floors below.
        let Some(found) = spots::under(world, layer, [x, y, z], top == Medium::Water) else { continue };
        let Some(entries) = list_at(world, layer, floor(&found)) else { continue };
        let entry = lists::pick(entries, fraction(world));
        // `canSpawnInConditions` gets the attempt column's water and lava, not the spot's; a refusal ends the pass (1415ef8b2).
        if matches!(top, Medium::Water | Medium::Lava) && top != entry.medium {
            break;
        }
        let size = herd_size(entry.herd, fraction(world));
        if !position_ok(world, layer, &found, entry, watchers) {
            break;
        }
        herd(world, layer, &found, entry, size);
    }
}

/// `isSpawnPositionOk` for the pass: a player between [`NEAREST`] and the far distance, and the
/// spot in water exactly when the entry spawns in water.
fn position_ok(world: &World, layer: Entity, found: &Found, entry: &Entry, watchers: &[Watcher]) -> bool {
    let far = if crate::ticking::tick_distance(world) <= CAPPED_TICK_RANGE { TICK_RANGE_FAR } else { FARTHEST };
    let nearest = watchers.iter().filter(|watcher| watcher.layer == layer).map(|watcher| watcher.distance(found.cell.map(|c| c as f32))).fold(f32::INFINITY, f32::min);
    (NEAREST..=far).contains(&nearest) && (found.medium == Medium::Water) == (entry.medium == Medium::Water)
}

/// Puts up to `size` of `entry` on `found` within its own caps, the 200 and its category's cap; each
/// member then passes its kind's own checks or does not come (`Spawner::spawnMob`, 14118caf0).
fn herd(world: &mut World, layer: Entity, found: &Found, entry: &Entry, size: u32) {
    let counts = count(world, layer, ChunkPos::of_block(found.cell[0], found.cell[2]));
    let place = usize::from(!entry.surface);
    let mut size = size.min(ALL.saturating_sub(counts.all) as u32);
    if let Some(caps) = entry.own_caps {
        size = size.min(caps[place].saturating_sub(counts.of_kind(entry.kind, place)));
    }
    let cap = CAPS.iter().find(|(name, ..)| *name == entry.category).map_or(0, |&(_, surface, underground)| if entry.surface { surface } else { underground });
    let mut held = counts.categories.get(entry.category).map_or(0, |held| held[place]);
    let spot = Found { surface: entry.surface, ..*found };
    for _ in 0..size {
        // BDS asks the cap before each member, `held <= cap`, so a category ends one over it.
        if held <= cap && entry.checks.pass(found.brightness, found.weathered) && spawn(world, layer, entry.kind, &spot, found.cell.map(|c| c as f32), entry.event) {
            held += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(bounds: [i32; 6], y_offset: i32) -> SpawnArea {
        SpawnArea { structure: "minecraft:swamp_hut".into(), bounds, whole: false, y_offset: Some(y_offset) }
    }

    #[test]
    fn a_hut_piece_starts_three_below_its_top_at_its_middle() {
        assert_eq!(middle(&piece([3840, 70, -1824, 3846, 76, -1816], -3)), [3843, 73, -1820]);
        assert!(contains(&piece([0, 0, 0, 1, 1, 1], 0), [1, 1, 1]) && !contains(&piece([0, 0, 0, 1, 1, 1], 0), [2, 1, 1]));
    }
}
