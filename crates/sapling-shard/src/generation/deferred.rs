//! What `/place structure` and `/place jigsaw` put into columns not loaded, kept until each loads,
//! as BDS's `PlaceCommand` `_placeOrDeferSections` does: every piece is cut to the chunks its box
//! touches and each cut placed when its chunk is loaded (Windows 143aee1f0, its lambda 143b313b0
//! queuing a one-chunk load action).

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ChunkPos, PlacedStructure};

use crate::blocks;
use crate::components::Layer;

/// One column's share of a placement.
#[derive(Default)]
struct Share {
    blocks: Vec<(BlockPos, u32)>,
    entities: Vec<(BlockPos, Value)>,
    actors: Vec<sapling_worldgen::TemplateActor>,
    dry: bool,
}

/// The shares of columns still to load, by world and column, oldest placement first.
#[derive(Resource, Default)]
pub(crate) struct Deferred(HashMap<(Entity, ChunkPos), Vec<Share>>);

fn column_of([x, _, z]: BlockPos) -> ChunkPos {
    ChunkPos::of_block(x, z)
}

/// Writes the shares of `placed` whose columns are loaded and keeps the others; `dry` takes
/// layer-1 water out of what it writes, `with_actors` makes its mobs.
pub(super) fn place_or_defer(world: &mut World, layer: Entity, placed: PlacedStructure, dry: bool, with_actors: bool) {
    let mut shares: HashMap<ChunkPos, Share> = HashMap::new();
    fn share(shares: &mut HashMap<ChunkPos, Share>, column: ChunkPos, dry: bool) -> &mut Share {
        shares.entry(column).or_insert_with(|| Share { dry, ..Share::default() })
    }
    for (pos, id) in placed.blocks {
        share(&mut shares, column_of(pos), dry).blocks.push((pos, id));
    }
    for (pos, entity) in placed.entities {
        share(&mut shares, column_of(pos), dry).entities.push((pos, entity));
    }
    for actor in placed.actors.into_iter().filter(|_| with_actors) {
        share(&mut shares, ChunkPos::of_block(actor.feet[0].floor() as i32, actor.feet[2].floor() as i32), dry).actors.push(actor);
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let (now, later): (Vec<_>, Vec<_>) = shares.into_iter().partition(|(column, _)| level.is_loaded(*column));
    let mut deferred = world.get_resource_or_insert_with(Deferred::default);
    for (column, share) in later {
        deferred.0.entry((layer, column)).or_default().push(share);
    }
    let mut now: Vec<Share> = now.into_iter().map(|(_, share)| share).collect();
    // One batch for every loaded column, as one placement is told.
    let blocks: Vec<(BlockPos, u32)> = now.iter_mut().flat_map(|share| std::mem::take(&mut share.blocks)).collect();
    write(world, layer, Share { blocks, dry, ..Share::default() });
    now.into_iter().for_each(|share| write(world, layer, share));
}

fn write(world: &mut World, layer: Entity, share: Share) {
    blocks::set_many(world, layer, &share.blocks);
    for (pos, _) in share.blocks.iter().filter(|_| share.dry) {
        blocks::clear_liquid(world, layer, *pos);
    }
    for (pos, entity) in &share.entities {
        blocks::keep_entity(world, layer, *pos, entity);
    }
    for actor in &share.actors {
        let event = Some(actor.event).filter(|event| *event != "minecraft:entity_spawned");
        crate::creatures::spawn_as(world, layer, actor.kind, actor.feet, sapling_api::SpawnReason::Structure, 0.0, event);
    }
}

/// Places what was kept for the column `home` of `layer`, which has just loaded.
pub(crate) fn arrived(world: &mut World, layer: Entity, home: ChunkPos) {
    let Some(shares) = world.get_resource_mut::<Deferred>().and_then(|mut deferred| deferred.0.remove(&(layer, home))) else { return };
    shares.into_iter().for_each(|share| write(world, layer, share));
}
