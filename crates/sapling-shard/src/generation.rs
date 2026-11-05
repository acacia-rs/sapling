//! The world generator on demand for `/place` and `/locate` (sapling_api::GenerationBackend):
//! what it places is written over the loaded columns as a batch, and what it finds is named by
//! the block BDS names (research/captures/bds-locate.txt).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GenerationBackend, PlaceError, PlaceOptions};
use sapling_world::{ChunkPos, PlaceFailure, PlacedStructure};

use crate::blocks::{self, Dice};
use crate::components::Layer;
use crate::handlers::Ctx;
use crate::worlds;

/// Writes what the generator placed (without layer-1 water under `options.ignore_waterlogging`), then the block entities of what it placed and, with
/// `options.include_entities`, its mobs, or says why it placed nothing.
// BDS made a town centre's golem with `doMobSpawning` off (bds-k1jig.txt), unlike a generated column's.
fn write(world: &mut World, layer: Entity, placed: Option<Result<PlacedStructure, PlaceFailure>>, options: PlaceOptions) -> Result<(), PlaceError> {
    let placed = match placed {
        None => return Err(PlaceError::NoGenerator),
        Some(Err(PlaceFailure::Unknown)) => return Err(PlaceError::Unknown),
        Some(Err(_)) => return Err(PlaceError::Failed),
        Some(Ok(placed)) => placed,
    };
    let level = &world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
    if !placed.blocks.iter().all(|([x, _, z], _)| level.is_loaded(ChunkPos::of_block(*x, *z))) {
        return Err(PlaceError::Unloaded);
    }
    blocks::set_many(world, layer, &placed.blocks);
    for (pos, _) in placed.blocks.iter().filter(|_| options.ignore_waterlogging) {
        blocks::clear_liquid(world, layer, *pos);
    }
    for (pos, entity) in &placed.entities {
        blocks::keep_entity(world, layer, *pos, entity);
    }
    for actor in placed.actors.iter().filter(|_| options.include_entities) {
        let event = Some(actor.event).filter(|event| *event != "minecraft:entity_spawned");
        crate::creatures::spawn_as(world, layer, actor.kind, actor.feet, sapling_api::SpawnReason::Structure, 0.0, event);
    }
    Ok(())
}

impl GenerationBackend for Ctx<'_> {
    fn place_feature(&mut self, name: &str, at: BlockPos) -> Result<(), PlaceError> {
        let layer = worlds::current(self.world);
        let seed = self.world.get_resource_or_insert_with(Dice::default).roll(u32::MAX);
        let level = &self.world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
        let placed = match level.place_feature(name, at, seed) {
            None => Err(PlaceFailure::Unknown),
            Some(writes) if writes.is_empty() => Err(PlaceFailure::Failed),
            Some(blocks) => Ok(PlacedStructure { blocks, ..PlacedStructure::default() }),
        };
        write(self.world, layer, Some(placed), PlaceOptions::default())
    }

    fn place_structure(&mut self, name: &str, at: BlockPos, options: PlaceOptions) -> Result<(), PlaceError> {
        let layer = worlds::current(self.world);
        let level = &self.world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
        let placed = level.place_structure(name, ChunkPos::of_block(at[0], at[2]), options.ignore_start_height.then_some(at[1]), options.keep_jigsaws);
        write(self.world, layer, placed, options)
    }

    fn place_jigsaw(&mut self, pool: &str, target: &str, max_depth: u32, at: BlockPos, options: PlaceOptions) -> Result<(), PlaceError> {
        let layer = worlds::current(self.world);
        let level = &self.world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
        let placed = level.place_jigsaw(pool, target, max_depth, at, options.keep_jigsaws);
        write(self.world, layer, placed, options)
    }

    fn locate_structure(&mut self, name: &str, from: BlockPos) -> Option<[i32; 2]> {
        let level = &self.world.get::<Layer>(worlds::current(self.world))?.world;
        level.locate_structure(name, from[0], from[2])
    }

    fn locate_biome(&mut self, name: &str, from: BlockPos) -> Option<BlockPos> {
        let level = &self.world.get::<Layer>(worlds::current(self.world))?.world;
        level.locate_biome(sapling_data::biome_id(name)?, from)
    }
}
