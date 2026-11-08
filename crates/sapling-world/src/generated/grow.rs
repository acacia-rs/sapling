//! A worldgen feature placed over the loaded columns, as a sapling grows its tree
//! (`sapling_worldgen::place_feature`), and the generator's structures placed the same way.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

use acacia_proto::nbt::Value;
use acacia_world::BlockRegistry;
use sapling_worldgen::{Block, BlockEntity, ChunkShape, HEIGHT, MIN_Y, PlaceFailure, StructureWrites};

use super::convert::BlockIds;
use super::entities::block_entity_nbt;
use crate::{ChunkPos, World};

/// What a runtime id is to worldgen, and back.
struct Ids {
    forward: BlockIds,
    back: HashMap<u32, Block>,
}

/// Built from the first registry asked: every world holds the vanilla registry.
fn ids(registry: &BlockRegistry) -> &'static Ids {
    static IDS: OnceLock<Ids> = OnceLock::new();
    IDS.get_or_init(|| {
        let forward = BlockIds::of(registry);
        let mut back = HashMap::new();
        Block::ALL.iter().chain(&Block::templates().collect::<Vec<_>>()).for_each(|&block| _ = back.entry(forward.id(block)).or_insert(block));
        Ids { forward, back }
    })
}

/// What a structure drawn over the loaded columns writes: the cells it changed with their new
/// runtime ids, the block entities of what it placed as BDS saves them, and the mobs its templates name.
#[derive(Debug, Default)]
pub struct PlacedStructure {
    pub blocks: Vec<([i32; 3], u32)>,
    pub entities: Vec<([i32; 3], Value)>,
    pub actors: Vec<sapling_worldgen::TemplateActor>,
}

impl PlacedStructure {
    /// With `kept` (`keepJigsaws`), its jigsaws written over what they would leave, in that registry's ids.
    fn of(writes: StructureWrites, ids: &Ids, kept: Option<&BlockRegistry>) -> Self {
        let named: HashMap<[i32; 3], Block> = writes.blocks.iter().copied().collect();
        let block = |at: [i32; 3]| named.get(&at).map_or("", |b| b.name());
        let mut entities: Vec<_> = writes.entities.iter().filter_map(|&(at, entity)| Some((at, block_entity_nbt(at, block(at), entity)?))).collect();
        let mut jigsaws = Vec::new();
        for &(at, entity) in writes.jigsaws.iter().filter(|_| kept.is_some()) {
            let BlockEntity::Jigsaw { facing, .. } = entity else { continue };
            // BDS's kept jigsaws all came with rotation 0, villages' vertical ones too (bds-k1jig.txt).
            let id = kept.and_then(|registry| registry.find("minecraft:jigsaw", &format!("facing_direction={facing},rotation=0")));
            if let (Some(id), Some(nbt)) = (id, block_entity_nbt(at, "jigsaw", entity)) {
                jigsaws.push((at, id));
                entities.push((at, nbt));
            }
        }
        let replaced: HashSet<[i32; 3]> = jigsaws.iter().map(|(at, _)| *at).collect();
        let mut blocks: Vec<_> = writes.blocks.iter().filter(|(pos, _)| !replaced.contains(pos)).map(|&(pos, block)| (pos, ids.forward.id(block))).collect();
        blocks.extend(jigsaws);
        Self { blocks, entities, actors: writes.actors }
    }
}

impl World {
    /// Places the feature `name` at `at` with a generator seeded `seed` over the loaded columns:
    /// the cells it writes and their new runtime ids, in the order it wrote them; `None` where it
    /// did not place. Columns not loaded read as air.
    // TODO: a block worldgen does not name reads as stone; layer 1 and biomes are not read.
    pub fn place_feature(&self, name: &str, at: [i32; 3], seed: u32) -> Option<Vec<([i32; 3], u32)>> {
        let ids = ids(&self.registry);
        let load = |chunk_x: i32, chunk_z: i32| Arc::new(self.shape_of(ChunkPos::new(chunk_x, chunk_z), ids));
        let writes = sapling_worldgen::place_feature(name, at, seed, &load, &|_| None)?;
        Some(writes.into_iter().map(|(pos, block, _)| (pos, ids.forward.id(block))).collect())
    }

    /// [`World::place_feature`] for the feature rule `name` run in `at`'s chunk as `/place
    /// featurerule` runs it (`sapling_worldgen::place_feature_rule`); empty where it placed nothing.
    pub fn place_feature_rule(&self, name: &str, at: [i32; 3], seed: u32) -> Option<Vec<([i32; 3], u32)>> {
        let ids = ids(&self.registry);
        let load = |chunk_x: i32, chunk_z: i32| Arc::new(self.shape_of(ChunkPos::new(chunk_x, chunk_z), ids));
        let writes = sapling_worldgen::place_feature_rule(name, at, seed, &load, &|_| None)?;
        Some(writes.into_iter().map(|(pos, block, _)| (pos, ids.forward.id(block))).collect())
    }

    /// The generator's structure `name` drawn over the loaded columns as `/place structure` starts
    /// it at the block `at`, its start height kept with `keep_height` (`ignoreStartHeight`) and its
    /// jigsaw blocks kept with `keep_jigsaws`; `None` without a generator.
    pub fn place_structure(&self, name: &str, at: [i32; 3], keep_height: bool, keep_jigsaws: bool) -> Option<Result<PlacedStructure, PlaceFailure>> {
        let ids = ids(&self.registry);
        let load = |chunk_x: i32, chunk_z: i32| Arc::new(self.shape_of(ChunkPos::new(chunk_x, chunk_z), ids));
        let writes = self.loader.as_ref()?.storage().structure_writes(name, at, keep_height, &load)?;
        Some(writes.map(|writes| PlacedStructure::of(writes, ids, keep_jigsaws.then_some(&self.registry))))
    }

    /// [`World::place_structure`] for a jigsaw pool assembled from `at`, as `/place jigsaw` does.
    pub fn place_jigsaw(&self, pool: &str, target: &str, max_depth: u32, at: [i32; 3], keep_jigsaws: bool) -> Option<Result<PlacedStructure, PlaceFailure>> {
        let ids = ids(&self.registry);
        let load = |chunk_x: i32, chunk_z: i32| Arc::new(self.shape_of(ChunkPos::new(chunk_x, chunk_z), ids));
        let writes = self.loader.as_ref()?.storage().jigsaw_writes(pool, target, max_depth, at, &load)?;
        Some(writes.map(|writes| PlacedStructure::of(writes, ids, keep_jigsaws.then_some(&self.registry))))
    }

    /// The block at a cell as the generator names it, its layer-1 water included; `None` for a
    /// state the generator never places.
    pub fn generator_block(&self, x: i32, y: i32, z: i32) -> Option<Block> {
        let ids = ids(&self.registry);
        let block = *ids.back.get(&self.block(x, y, z))?;
        let wet = ids.back.get(&self.liquid(x, y, z)) == Some(&Block::Water);
        Some(if wet { block.into_water() } else { block })
    }

    /// The runtime id of a block the generator names (layer 0: a waterlogged twin's dry block).
    pub fn generator_id(&self, block: Block) -> u32 {
        ids(&self.registry).forward.id(block.dry())
    }

    fn shape_of(&self, pos: ChunkPos, ids: &Ids) -> ChunkShape {
        let mut shape = ChunkShape::empty();
        if !self.is_loaded(pos) {
            return shape;
        }
        for y in MIN_Y..MIN_Y + HEIGHT as i32 {
            for z in 0..16 {
                for x in 0..16 {
                    let id = self.block(pos.x * 16 + x as i32, y, pos.z * 16 + z as i32);
                    if id != self.dim.air {
                        shape.set_block(x, y, z, ids.back.get(&id).copied().unwrap_or(Block::Stone));
                    }
                }
            }
        }
        shape
    }
}
