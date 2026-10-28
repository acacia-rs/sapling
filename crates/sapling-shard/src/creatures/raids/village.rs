//! Where a village is, as sapling takes it (docs/research/effects-raids.md "Villages"): its
//! middle is the nearest bell within [`FINDS`] across, else the nearest bed, and it holds while a
//! villager lives within [`HOLDS`] of that middle.
// TODO: BDS's villages are their dwellers' claimed beds, bells and job sites (`Village`); not drilled.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

use crate::body::Body;
use crate::components::{InLayer, Layer};
use crate::entities::Kind;

const FINDS: f32 = 48.0;
const HOLDS: f32 = 64.0;
const VILLAGERS: [&str; 2] = ["minecraft:villager_v2", "minecraft:villager"];

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// The middle of the village about `feet` in `layer`, if there is one.
pub(in crate::creatures) fn centre(world: &mut World, layer: Entity, feet: [f32; 3]) -> Option<[f32; 3]> {
    let level = &world.get::<Layer>(layer)?.world;
    let reach = (FINDS / 16.0).ceil() as i32;
    let home = ChunkPos::of_block(feet[0].floor() as i32, feet[2].floor() as i32);
    let mut found: Vec<(bool, f32, [f32; 3])> = Vec::new();
    for column in (-reach..=reach).flat_map(|dx| (-reach..=reach).map(move |dz| ChunkPos::new(home.x + dx, home.z + dz))) {
        for entity in level.block_entities_in(column) {
            let bell = match entity.nbt.get("id") {
                Some(Value::String(id)) if id == "Bell" => true,
                Some(Value::String(id)) if id == "Bed" => false,
                _ => continue,
            };
            let at = entity.pos.map(|c| c as f32 + 0.5);
            let off = distance(at, feet);
            if off <= FINDS {
                found.push((!bell, off, at));
            }
        }
    }
    found.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).unwrap_or(std::cmp::Ordering::Equal));
    let middle = found.first()?.2;
    holds(world, layer, middle).then_some(middle)
}

/// Whether a villager lives within reach of the village's middle; true while that middle is not
/// loaded, as a kept raid waits for its village.
pub(super) fn holds(world: &mut World, layer: Entity, middle: [f32; 3]) -> bool {
    let chunk = ChunkPos::of_block(middle[0].floor() as i32, middle[2].floor() as i32);
    if world.get::<Layer>(layer).is_some_and(|kept| !kept.world.is_loaded(chunk)) {
        return true;
    }
    let near: Vec<Entity> = world
        .query::<(Entity, &InLayer, &Kind, &Body)>()
        .iter(world)
        .filter(|(_, on, kind, body)| on.0 == layer && VILLAGERS.contains(&kind.0.as_str()) && distance(body.at, middle) <= HOLDS)
        .map(|(villager, ..)| villager)
        .collect();
    near.into_iter().any(|villager| super::super::is_alive(world, villager))
}
