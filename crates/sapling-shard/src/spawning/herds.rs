//! One spawn attempt in a ticked chunk (BDS's `BedrockSpawner::tick`): a random column, a cluster
//! at each of its levels (spots.rs). A cluster picks, by weight, a rule its spot passes and puts a
//! herd of that kind on the spot, all on the one cell, as many as the caps leave room for.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, SpawnReason};
use sapling_data::{SpawnRule, spawn_category, spawn_rules};
use sapling_world::ChunkPos;

use super::{gates, phantoms};
use super::spots::{self, Found};
use super::{Natural, SpawnDice, Watcher};
use crate::body::Body;
use crate::components::{InLayer, Layer, Settings, Tick};
use crate::creatures::Creature;
use crate::entities::{self, Kind};

/// How far round a cluster's chunk, in chunks, the mobs counted against its caps are (`_updateBaseTypeCount`).
const COUNTED: i32 = 4;
/// The most spawnable mobs a world holds before none more appear.
const ALL: usize = 200;

/// A category's caps of mobs about a cluster on the surface and underground.
// TODO: BDS reads these off the dimension (`region vfunc 0x128`); the numbers are not found yet.
const CAPS: &[(&str, usize, usize)] = &[("monster", 8, 8), ("animal", 4, 4), ("water_animal", 5, 5), ("ambient", 2, 2)];

fn difficulty(world: &World) -> u8 {
    match world.resource::<Settings>().0.difficulty {
        Difficulty::Peaceful => 0,
        Difficulty::Easy => 1,
        Difficulty::Normal => 2,
        Difficulty::Hard => 3,
    }
}

/// A roll from 0 to 1 of natural spawning's own dice.
pub(super) fn fraction(world: &mut World) -> f32 {
    world.get_resource_or_insert_with(SpawnDice::default).0.fraction()
}

/// The mobs about a cluster, by category and by kind, each split by surface (`[surface, underground]`).
#[derive(Default)]
struct Counts {
    categories: HashMap<&'static str, [usize; 2]>,
    kinds: HashMap<String, [usize; 2]>,
    all: usize,
}

fn count(world: &mut World, layer: Entity, around: ChunkPos) -> Counts {
    let mut mobs = world.query_filtered::<(&InLayer, &Kind, &Body), With<Creature>>();
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return Counts::default() };
    let mut counts = Counts::default();
    for (_, kind, body) in mobs.iter(world).filter(|(in_layer, ..)| in_layer.0 == layer) {
        counts.all += 1;
        let [x, y, z] = body.at.map(|c| c.floor() as i32);
        let column = ChunkPos::of_block(x, z);
        if (column.x - around.x).abs() > COUNTED || (column.z - around.z).abs() > COUNTED {
            continue;
        }
        let place = usize::from(level.top_solid(x, z).is_some_and(|top| y <= top));
        if let Some(category) = spawn_category(&kind.0) {
            counts.categories.entry(category).or_default()[place] += 1;
        }
        counts.kinds.entry(kind.0.clone()).or_default()[place] += 1;
    }
    counts
}

/// An attempt in `chunk` of `layer`: a cluster at each level of one of its columns.
pub(super) fn attempt(world: &mut World, layer: Entity, chunk: ChunkPos, watchers: &[Watcher]) {
    let [dx, dz] = [fraction(world), fraction(world)].map(|roll| (roll * 16.0) as i32);
    for found in spots::levels(world, layer, chunk.x * 16 + dx, chunk.z * 16 + dz) {
        cluster(world, layer, &found, watchers);
    }
}

fn cluster(world: &mut World, layer: Entity, found: &Found, watchers: &[Watcher]) -> Option<()> {
    let distance = watchers.iter().filter(|watcher| watcher.layer == layer).map(|watcher| watcher.distance(found.feet())).fold(f32::INFINITY, f32::min);
    let spot = found.spot(difficulty(world), distance);
    let mut admitted: Vec<&SpawnRule> = spawn_rules().iter().filter(|rule| rule.admits(&spot)).collect();
    admitted.retain(|rule| gates::admits(world, layer, rule.kind, found));
    let rule = pick(&admitted, fraction(world))?;
    let place = usize::from(!found.surface);
    let counts = count(world, layer, ChunkPos::of_block(found.cell[0], found.cell[2]));
    let cap = CAPS.iter().find(|(name, ..)| *name == rule.category).map(|&(_, surface, underground)| if found.surface { surface } else { underground })?;
    if counts.categories.get(rule.category).map_or(0, |held| held[place]) >= cap {
        return None;
    }
    let of_kind = counts.kinds.get(rule.kind).map_or(0, |held| held[place]) as u32;
    let room = rule.density_at(found.surface).map_or(u32::MAX, |density| density.saturating_sub(of_kind)).min(ALL.saturating_sub(counts.all) as u32);
    let size = (rule.herd.0 + (fraction(world) * (rule.herd.1 - rule.herd.0 + 1) as f32) as u32).min(rule.herd.1).min(room);
    for _ in 0..size {
        let kind = rule.permuted(fraction(world));
        let place = if kind == phantoms::PHANTOM { phantoms::place(world, layer, found) } else { Some(*found) };
        if let Some(place) = place {
            spawn(world, layer, kind, &place);
        }
    }
    Some(())
}

/// A rule of `admitted` by their weights, by `roll` from 0 to 1.
fn pick<'a>(admitted: &[&'a SpawnRule], roll: f32) -> Option<&'a SpawnRule> {
    let total: u32 = admitted.iter().map(|rule| rule.weight).sum();
    let mut left = (roll * total as f32) as u32;
    for rule in admitted {
        if left < rule.weight {
            return Some(rule);
        }
        left -= rule.weight;
    }
    None
}

/// A mob of `kind` at `place`, where it fits, that despawns.
fn spawn(world: &mut World, layer: Entity, kind: &str, place: &Found) {
    let Some([_, height]) = sapling_data::mob_size(kind) else { return };
    if !world.get::<Layer>(layer).is_some_and(|level| spots::has_room(&level.world, place.cell, height)) {
        return;
    }
    let Some(id) = crate::creatures::spawn(world, layer, kind, place.feet(), SpawnReason::Natural) else { return };
    let now = world.resource::<Tick>().0;
    if let Some(entity) = entities::find(world, id) {
        world.entity_mut(entity).insert(Natural { born: now });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_are_picked_by_weight() {
        let rules: Vec<&SpawnRule> = spawn_rules().iter().filter(|rule| rule.kind == "minecraft:zombie" || rule.kind == "minecraft:witch").collect();
        let total: u32 = rules.iter().map(|rule| rule.weight).sum();
        let first = rules[0].weight as f32 / total as f32;
        assert_eq!(pick(&rules, 0.0).map(|rule| rule.kind), Some(rules[0].kind));
        assert_eq!(pick(&rules, first + 0.001).map(|rule| rule.kind), Some(rules[1].kind));
        assert!(pick(&[], 0.5).is_none());
    }
}
