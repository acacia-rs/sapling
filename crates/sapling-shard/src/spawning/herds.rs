//! One spawn attempt in a ticked chunk (BDS's `BedrockSpawner::tick`): a random column, a cluster
//! at each of its levels (spots.rs). A cluster picks, by weight, a rule its spot passes and puts a
//! herd of that kind on the spot, all on the one cell, as many as the caps leave room for.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, SpawnReason};
use sapling_data::{HerdEvents, SpawnRule, spawn_category, spawn_rules};
use sapling_world::ChunkPos;

use super::{gates, phantoms, structures};
use sapling_data::Medium;
use super::spots::{self, Found};
use super::{Natural, SpawnDice, Watcher};
use crate::body::Body;
use crate::components::{InLayer, Layer, Settings};
use crate::creatures::Creature;
use crate::entities::{self, Kind};

/// How far round a cluster's chunk, in chunks, the mobs counted against its caps are (`_updateBaseTypeCount`).
const COUNTED: i32 = 4;
/// The most spawnable mobs a world holds before none more appear.
pub(super) const ALL: usize = 200;

/// A category's caps of mobs about a cluster on the surface and underground: the base `Dimension`
/// constructor's (141878200, +0x104 and +0x120), so every dimension's (mob-spawning.md "Caps").
pub(super) const CAPS: &[(&str, usize, usize)] = &[("animal", 4, 4), ("monster", 8, 16), ("water_animal", 36, 36), ("ambient", 0, 2)];
/// The herd events a natural mob is made by here (sapling_data::HerdEvents): babies of fox, axolotl
/// and polar bear herds.
// TODO: the other herd events (horse coats, tropical fish looks, drowned and husk riders) need the
// event fired on a spawned mob; a summon's event replaces `entity_spawned` instead.
const FIRED: [&str; 1] = ["minecraft:entity_born"];

pub(super) fn difficulty(world: &World) -> u8 {
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
pub(super) struct Counts {
    pub categories: HashMap<&'static str, [usize; 2]>,
    kinds: HashMap<String, [usize; 2]>,
    pub all: usize,
}

pub(super) fn count(world: &mut World, layer: Entity, around: ChunkPos) -> Counts {
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
        if let Some(category) = spawn_category(&kind.0).or_else(|| structures::category(&kind.0)) {
            counts.categories.entry(category).or_default()[place] += 1;
        }
        counts.kinds.entry(kind.0.clone()).or_default()[place] += 1;
    }
    counts
}

/// An attempt in `chunk` of `layer`: a cluster at each level of one of its columns but those in a
/// structure's piece, then the structure pass (structures.rs) if the column has a spot.
pub(super) fn attempt(world: &mut World, layer: Entity, chunk: ChunkPos, watchers: &[Watcher]) {
    let [dx, dz] = [fraction(world), fraction(world)].map(|roll| (roll * 16.0) as i32);
    let levels = spots::levels(world, layer, chunk.x * 16 + dx, chunk.z * 16 + dz);
    for found in &levels {
        if !structures::in_piece(world, layer, found) {
            cluster(world, layer, found, watchers);
        }
    }
    if let Some(top) = levels.first() {
        structures::attempt(world, layer, chunk, top.medium == Medium::Water);
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
    let mut held = counts.categories.get(rule.category).map_or(0, |held| held[place]);
    let of_kind = counts.kinds.get(rule.kind).map_or(0, |held| held[place]) as u32;
    let room = rule.density_at(found.surface).map_or(u32::MAX, |density| density.saturating_sub(of_kind)).min(ALL.saturating_sub(counts.all) as u32);
    let herd = if rule.herds.is_empty() { None } else { pick_herd(&rule.herds, fraction(world)) };
    let size = herd_size(rule.herd, fraction(world)).min(room);
    let mut member = 0;
    // BDS asks the cap before each member, `held <= cap`, so a category ends one over it (1415ef8b2 on).
    for _ in 0..size {
        if held > cap {
            break;
        }
        let kind = rule.permuted(fraction(world));
        let place = if kind == phantoms::PHANTOM { phantoms::place(world, layer, found) } else { Some(*found) };
        let event = herd.and_then(|herd| herd.events_of(member).find(|event| FIRED.contains(event)));
        if place.is_some_and(|place| spawn(world, layer, kind, &place, place.feet(), event)) {
            held += 1;
            member += 1;
        }
    }
    Some(())
}

/// A herd's size between `least` and `most` by `roll` from 0 to 1, BDS's `least + roundf(roll² (most - least))`
/// (`_spawnMobCluster` 1415ed230), so small herds are the likelier.
pub(super) fn herd_size((least, most): (u32, u32), roll: f32) -> u32 {
    least + (roll * roll * most.saturating_sub(least) as f32).round() as u32
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

/// One of a rule's herds by `roll` from 0 to 1, as BDS's `nextInt(herds)`.
fn pick_herd(herds: &[HerdEvents], roll: f32) -> Option<&HerdEvents> {
    herds.get(((roll * herds.len() as f32) as usize).min(herds.len().saturating_sub(1)))
}

/// A mob of `kind` with its feet `at`, if it fits in `place`, that despawns, as `event` makes it;
/// whether one came.
pub(super) fn spawn(world: &mut World, layer: Entity, kind: &str, place: &Found, at: [f32; 3], event: Option<&str>) -> bool {
    let Some([_, height]) = sapling_data::mob_size(kind) else { return false };
    if !world.get::<Layer>(layer).is_some_and(|level| spots::has_room(&level.world, place.cell, height)) {
        return false;
    }
    let Some(id) = crate::creatures::spawn_as(world, layer, kind, at, SpawnReason::Natural, 0.0, event) else { return false };
    if let Some(entity) = entities::find(world, id) {
        world.entity_mut(entity).insert(Natural);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fox_herd_is_two_grown_then_babies() {
        let Some(fox) = spawn_rules().iter().find(|rule| rule.kind == "minecraft:fox") else { return };
        let herd = pick_herd(&fox.herds, 0.99).expect("spawn-rules.tsv from the current spawn-rules.py");
        let fired: Vec<Option<&str>> = (0..4).map(|member| herd.events_of(member).find(|event| FIRED.contains(event))).collect();
        assert_eq!(fired, [None, None, Some("minecraft:entity_born"), Some("minecraft:entity_born")]);
        assert!(pick_herd(&[], 0.5).is_none());
    }

    #[test]
    fn rules_are_picked_by_weight() {
        let rules: Vec<&SpawnRule> = spawn_rules().iter().filter(|rule| rule.kind == "minecraft:zombie" || rule.kind == "minecraft:witch").collect();
        let total: u32 = rules.iter().map(|rule| rule.weight).sum();
        let first = rules[0].weight as f32 / total as f32;
        assert_eq!(pick(&rules, 0.0).map(|rule| rule.kind), Some(rules[0].kind));
        assert_eq!(pick(&rules, first + 0.001).map(|rule| rule.kind), Some(rules[1].kind));
        assert!(pick(&[], 0.5).is_none());
    }

    #[test]
    fn herd_sizes_lean_small_as_bds_squares_the_roll() {
        assert_eq!([0.0, 0.5, 0.71, 0.99].map(|roll| herd_size((2, 4), roll)), [2, 3, 3, 4]);
        assert_eq!(herd_size((1, 1), 0.9), 1);
    }
}
