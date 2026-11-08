//! The world generator on demand for `/place` and `/locate` (sapling_api::GenerationBackend):
//! what it places is written over the loaded columns as a batch (a structure's share of columns
//! not loaded once they load, deferred.rs), and what it finds is named by the block BDS names
//! (research/captures/bds-locate.txt).

mod deferred;

pub(crate) use deferred::arrived as structures_arrived;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GenerationBackend, PlaceError, PlaceOptions};
use sapling_world::{ChunkPos, PlaceFailure, PlacedStructure};

use crate::blocks::{self, Dice};
use crate::components::Layer;
use crate::handlers::Ctx;
use crate::worlds;

fn placed_or_why(placed: Option<Result<PlacedStructure, PlaceFailure>>) -> Result<PlacedStructure, PlaceError> {
    match placed {
        None => Err(PlaceError::NoGenerator),
        Some(Err(PlaceFailure::Unknown)) => Err(PlaceError::Unknown),
        Some(Err(_)) => Err(PlaceError::Failed),
        Some(Ok(placed)) => Ok(placed),
    }
}

/// Writes a structure's or jigsaw's pieces over the loaded columns and keeps the rest until their
/// columns load (BDS placed parts of y3place's out of the loaded area, bds-y3place.txt), or says why
/// it placed nothing.
// BDS made a town centre's golem with `doMobSpawning` off (bds-k1jig.txt), unlike a generated column's.
fn write_structure(world: &mut World, layer: Entity, placed: Option<Result<PlacedStructure, PlaceFailure>>, options: PlaceOptions) -> Result<(), PlaceError> {
    let placed = placed_or_why(placed)?;
    world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?;
    deferred::place_or_defer(world, layer, placed, options.ignore_waterlogging, options.include_entities);
    Ok(())
}

/// Writes a feature's or a feature rule's blocks (None: not a known one; none: it placed nothing),
/// all over loaded columns, then furnishes the bee nests among them.
fn write_feature(world: &mut World, layer: Entity, writes: Option<Vec<(BlockPos, u32)>>) -> Result<(), PlaceError> {
    let placed = match writes {
        None => Err(PlaceError::Unknown),
        Some(writes) if writes.is_empty() => Err(PlaceError::Failed),
        Some(blocks) => Ok(blocks),
    }?;
    let level = &world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
    if !placed.iter().all(|([x, _, z], _)| level.is_loaded(ChunkPos::of_block(*x, *z))) {
        return Err(PlaceError::Unloaded);
    }
    deferred::place_or_defer(world, layer, PlacedStructure { blocks: placed.clone(), ..PlacedStructure::default() }, false, false);
    blocks::furnish_nests(world, layer, &placed);
    Ok(())
}

/// The seed every `/place featurerule` draws from: BDS's level random's, drawn once a server run
/// (sapling_worldgen::place_feature_rule).
#[derive(Resource, Clone, Copy)]
struct RuleSeed(u32);

impl GenerationBackend for Ctx<'_> {
    fn place_feature(&mut self, name: &str, at: BlockPos) -> Result<(), PlaceError> {
        let layer = worlds::current(self.world);
        let seed = self.world.get_resource_or_insert_with(Dice::default).roll(u32::MAX);
        let level = &self.world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
        let writes = level.place_feature(name, at, seed);
        write_feature(self.world, layer, writes)
    }

    fn place_feature_rule(&mut self, name: &str, at: BlockPos) -> Result<(), PlaceError> {
        let layer = worlds::current(self.world);
        let seed = match self.world.get_resource::<RuleSeed>() {
            Some(seed) => seed.0,
            None => {
                let seed = RuleSeed(self.world.get_resource_or_insert_with(Dice::default).roll(u32::MAX));
                self.world.insert_resource(seed);
                seed.0
            }
        };
        let level = &self.world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
        let chunk = ChunkPos::of_block(at[0], at[2]);
        // BDS's `areChunksFullyLoaded(pos, 1)`: the chunk and the eight round it.
        if !(-1..=1).all(|dx| (-1..=1).all(|dz| level.is_loaded(ChunkPos::new(chunk.x + dx, chunk.z + dz)))) {
            return Err(PlaceError::Unloaded);
        }
        let writes = level.place_feature_rule(name, at, seed);
        write_feature(self.world, layer, writes)
    }

    fn place_structure(&mut self, name: &str, at: BlockPos, options: PlaceOptions) -> Result<(), PlaceError> {
        let layer = worlds::current(self.world);
        let level = &self.world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
        let placed = level.place_structure(name, at, options.ignore_start_height, options.keep_jigsaws);
        write_structure(self.world, layer, placed, options)
    }

    fn place_jigsaw(&mut self, pool: &str, target: &str, max_depth: u32, at: BlockPos, options: PlaceOptions) -> Result<(), PlaceError> {
        let layer = worlds::current(self.world);
        let level = &self.world.get::<Layer>(layer).ok_or(PlaceError::NoGenerator)?.world;
        let placed = level.place_jigsaw(pool, target, max_depth, at, options.keep_jigsaws);
        write_structure(self.world, layer, placed, options)
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
