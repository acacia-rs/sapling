//! The world generator on demand for `/place` and `/locate` (sapling_api::GenerationBackend):
//! what it places is written over the loaded columns as a batch, and what it finds is named by
//! the block BDS names (research/captures/bds-locate.txt).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GenerationBackend, PlaceError};
use sapling_world::{ChunkPos, PlaceFailure};

use crate::blocks::{self, Dice};
use crate::components::Layer;
use crate::handlers::Ctx;
use crate::worlds;

/// Writes what the generator placed, or says why it placed nothing.
fn write(world: &mut World, layer: Entity, placed: Option<Result<Vec<([i32; 3], u32)>, PlaceFailure>>) -> Result<(), PlaceError> {
    let writes = match placed {
        None => return Err(PlaceError::NoGenerator),
        Some(Err(PlaceFailure::Unknown)) => return Err(PlaceError::Unknown),
        Some(Err(_)) => return Err(PlaceError::Failed),
        Some(Ok(writes)) => writes,
    };
    let level = &world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
    if !writes.iter().all(|([x, _, z], _)| level.is_loaded(ChunkPos::of_block(*x, *z))) {
        return Err(PlaceError::Unloaded);
    }
    blocks::set_many(world, layer, &writes);
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
            Some(writes) => Ok(writes),
        };
        write(self.world, layer, Some(placed))
    }

    fn place_structure(&mut self, name: &str, at: BlockPos) -> Result<(), PlaceError> {
        let layer = worlds::current(self.world);
        let level = &self.world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
        let placed = level.place_structure(name, ChunkPos::of_block(at[0], at[2]));
        write(self.world, layer, placed)
    }

    fn place_jigsaw(&mut self, pool: &str, target: &str, max_depth: u32, at: BlockPos) -> Result<(), PlaceError> {
        let layer = worlds::current(self.world);
        let level = &self.world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
        let placed = level.place_jigsaw(pool, target, max_depth, at);
        write(self.world, layer, placed)
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
