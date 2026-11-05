//! Structure spawning, BDS's pass after a spawn attempt's column (`BedrockSpawner::tick` 1415eebf0):
//! the ticked chunk's spawn-area pieces of a structure with its own list (fortress, monument, swamp
//! hut, outpost) give their middles; a few tries each pick one, find a spot down from it and put a
//! herd of an entry of the list there. A natural cluster whose floor is in such a piece is skipped.
//! The areas are the column's saved ones (sapling_world::SpawnArea). See docs/research/structure-spawns.md.

use bevy_ecs::prelude::*;
use sapling_data::Medium;
use sapling_world::{ChunkPos, SpawnArea};

use super::herds::{ALL, CAPS, count, difficulty, fraction, herd_size, spawn};
use super::spots::{self, Found};
use crate::components::Layer;

/// One kind a structure's list spawns (registered by 141f82140, all in the monster category).
struct Entry {
    kind: &'static str,
    event: Option<&'static str>,
    herd: (u32, u32),
    medium: Medium,
    /// The brightest spot it takes; drilled: hut witches never came at noon, pillagers, guardians did.
    brightest: u8,
}

const fn entry(kind: &'static str, least: u32, most: u32) -> Entry {
    Entry { kind, event: None, herd: (least, most), medium: Medium::Air, brightest: 15 }
}

const FORTRESS: &[Entry] = &[
    entry("minecraft:zombie_pigman", 2, 3),
    entry("minecraft:magma_cube", 2, 3),
    entry("minecraft:blaze", 2, 3),
    entry("minecraft:skeleton", 2, 3),
    entry("minecraft:wither_skeleton", 2, 3),
];
const MONUMENT: &[Entry] = &[Entry { medium: Medium::Water, ..entry("minecraft:guardian", 2, 4) }];
const SWAMP_HUT: &[Entry] = &[Entry { brightest: 7, ..entry("minecraft:witch", 1, 1) }];
const OUTPOST: &[Entry] = &[entry("minecraft:pillager", 1, 2), Entry { event: Some("minecraft:spawn_as_illager_captain"), ..entry("minecraft:pillager", 1, 1) }];

fn list(structure: &str) -> Option<&'static [Entry]> {
    match structure {
        "minecraft:fortress" => Some(FORTRESS),
        "minecraft:monument" => Some(MONUMENT),
        "minecraft:swamp_hut" => Some(SWAMP_HUT),
        "minecraft:pillager_outpost" => Some(OUTPOST),
        _ => None,
    }
}

/// The category a structure's own kind counts in, for kinds no natural rule names (pillager, blaze, ...).
pub(super) fn category(kind: &str) -> Option<&'static str> {
    [FORTRESS, MONUMENT, SWAMP_HUT, OUTPOST].iter().flat_map(|list| list.iter()).any(|entry| entry.kind == kind).then_some("monster")
}

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

/// The structure pass of an attempt in `chunk`; `water` when the attempt's column starts under water.
pub(super) fn attempt(world: &mut World, layer: Entity, chunk: ChunkPos, water: bool) {
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
        let Some(found) = spots::under(world, layer, [x, y, z], water) else { continue };
        let Some(entries) = list_at(world, layer, floor(&found)) else { continue };
        let entry = &entries[roll(world, entries.len())];
        // A refused spot ends the pass, not just the try (1415ef8b2).
        if entry.medium != found.medium || found.brightness > entry.brightest || !herd(world, layer, &found, entry) {
            break;
        }
    }
}

/// Puts a herd of `entry` on `found` within the monster caps; false where none could stand there.
fn herd(world: &mut World, layer: Entity, found: &Found, entry: &Entry) -> bool {
    let size = herd_size(entry.herd, fraction(world));
    let Some([_, height]) = sapling_data::mob_size(entry.kind) else { return false };
    if !world.get::<Layer>(layer).is_some_and(|level| spots::has_room(&level.world, found.cell, height)) {
        return false;
    }
    let counts = count(world, layer, ChunkPos::of_block(found.cell[0], found.cell[2]));
    let place = usize::from(!found.surface);
    let cap = CAPS.iter().find(|(name, ..)| *name == "monster").map_or(0, |&(_, surface, underground)| if found.surface { surface } else { underground });
    let mut held = counts.categories.get("monster").map_or(0, |held| held[place]);
    let at = found.cell.map(|c| c as f32);
    for _ in 0..size.min(ALL.saturating_sub(counts.all) as u32) {
        if held > cap {
            break;
        }
        if spawn(world, layer, entry.kind, found, at, entry.event) {
            held += 1;
        }
    }
    true
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

    #[test]
    fn only_the_four_structures_have_lists() {
        let kinds = ["minecraft:fortress", "minecraft:monument", "minecraft:swamp_hut", "minecraft:pillager_outpost", "minecraft:trial_chambers"];
        assert_eq!(kinds.map(|kind| list(kind).map_or(0, <[Entry]>::len)), [5, 1, 1, 2, 0]);
        assert_eq!([category("minecraft:pillager"), category("minecraft:cow")], [Some("monster"), None]);
    }
}
