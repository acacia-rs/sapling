//! Which furnaces and hoppers tick. BDS ticks block actors only near players; here one ticks
//! while its column is in a player's view. Every [`UNLOAD_INTERVAL`] ticks, before columns are
//! saved, those in seen columns that have work start (a furnace that was burning when its world
//! was saved, any hopper), and those in columns nobody sees any more stop, their state kept in
//! their block entities. A placed hopper and an opened furnace start at once.

use std::collections::HashSet;

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ChunkPos;

use crate::chunks::UNLOAD_INTERVAL;
use crate::components::{InLayer, Layer, Tick};
use crate::view::View;
use crate::{furnaces, hoppers};

fn id(entity: &Value) -> Option<&str> {
    match entity.get("id") {
        Some(Value::String(id)) => Some(id),
        _ => None,
    }
}

/// The loaded columns some player's view covers, by layer.
fn seen(world: &mut World) -> HashSet<(Entity, ChunkPos)> {
    let (mut players, mut layers) = (world.query::<(&InLayer, &View)>(), world.query::<(Entity, &Layer)>());
    let world = &*world;
    let mut seen = HashSet::new();
    for (layer, level) in layers.iter(world) {
        let is_seen = |pos: &ChunkPos| players.iter(world).any(|(in_layer, view)| in_layer.0 == layer && view.covers(*pos));
        seen.extend(level.world.loaded().filter(is_seen).map(|pos| (layer, pos)));
    }
    seen
}

/// Starts and stops the furnaces and hoppers by who sees their columns; see the module docs.
pub(crate) fn sweep(world: &mut World) {
    if world.resource::<Tick>().0 % UNLOAD_INTERVAL != 0 {
        return;
    }
    let seen = seen(world);
    let column = |pos: BlockPos| ChunkPos::of_block(pos[0], pos[2]);
    let watched: Vec<(Entity, BlockPos)> = furnaces::watched(world).into_iter().chain(hoppers::watched(world)).collect();
    for (layer, pos) in watched.into_iter().filter(|(layer, pos)| !seen.contains(&(*layer, column(*pos)))) {
        furnaces::forget(world, layer, pos);
        hoppers::forget(world, layer, pos);
    }
    let mut found: Vec<(Entity, BlockPos, bool)> = Vec::new();
    for &(layer, pos) in &seen {
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { continue };
        for entity in level.block_entities_in(pos) {
            match id(&entity.nbt) {
                Some("Hopper") => found.push((layer, entity.pos, true)),
                Some("Furnace" | "BlastFurnace" | "Smoker") if furnaces::was_at_work(&entity.nbt) => found.push((layer, entity.pos, false)),
                _ => {}
            }
        }
    }
    for (layer, pos, is_hopper) in found {
        match is_hopper {
            true => hoppers::watch(world, layer, pos),
            false => furnaces::watch(world, layer, pos),
        }
    }
}
