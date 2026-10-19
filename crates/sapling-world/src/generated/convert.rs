//! Generated blocks and biomes as an acacia-world [`Chunk`] in the world's registry. Water and lava
//! go in layer 0, as BDS saves generated fluid, and a block's second layer (`Block::second_layer`) in layer 1;
//! sections that are all air are left out.

use acacia_world::{BlockRegistry, Chunk, Dimension, SECTION_VOLUME};
use sapling_worldgen::{Block, ChunkBiomes, ChunkShape};

/// The runtime id of every [`Block`] the generator places, looked up once by name and states.
pub(crate) struct BlockIds {
    ids: Vec<u32>,
    templates: Vec<u32>,
}

impl BlockIds {
    /// Panics when the registry lacks a generated state: the generator names vanilla states only.
    pub fn of(registry: &BlockRegistry) -> Self {
        let id = |block: Block| {
            let name = format!("minecraft:{}", block.name());
            registry.find(&name, block.states()).unwrap_or_else(|| panic!("the registry has no {name} [{}]", block.states()))
        };
        Self { ids: Block::ALL.iter().map(|&block| id(block)).collect(), templates: Block::templates().map(id).collect() }
    }

    pub(crate) fn id(&self, block: Block) -> u32 {
        match block {
            Block::Template(i) => self.templates[i as usize],
            _ => self.ids[block.index()],
        }
    }
}

/// The chunk at `(chunk_x, chunk_z)` holding `shape`'s blocks and `biomes`' biome ids.
pub(crate) fn generated_chunk(chunk_x: i32, chunk_z: i32, shape: &ChunkShape, biomes: &ChunkBiomes, ids: &BlockIds, dim: Dimension) -> Chunk {
    let mut chunk = Chunk::empty(chunk_x, chunk_z, dim);
    let mut blocks = [0u32; SECTION_VOLUME];
    let mut liquid = [dim.air; SECTION_VOLUME];
    for section in 0..chunk.section_count() as i32 {
        let base = dim.min_y + section * 16;
        let (mut solid, mut waterlogged) = (false, false);
        for (index, slot) in blocks.iter_mut().enumerate() {
            let (x, z, y) = (index >> 8, (index >> 4) & 15, base + (index & 15) as i32);
            let block = shape.block(x, y, z);
            *slot = ids.id(block);
            solid |= *slot != dim.air;
            let second = block.second_layer();
            liquid[index] = second.map_or(dim.air, |layer| ids.id(layer));
            waterlogged |= second.is_some();
        }
        if solid {
            chunk.set_section(base >> 4, &blocks, waterlogged.then_some(&liquid));
        }
    }
    for y in dim.min_y..dim.min_y + dim.height as i32 {
        for z in 0..16 {
            for x in 0..16 {
                chunk.set_biome(x as i32, y, z as i32, biomes.biome(x, y, z));
            }
        }
    }
    chunk
}

#[cfg(test)]
mod tests {
    use acacia_world::{BlockRegistry, Dimension};
    use sapling_worldgen::{Block, MIN_Y, OverworldGenerator};

    use super::{BlockIds, generated_chunk};

    /// Every block table of the generator (`Block::ALL`) and the whole template palette.
    #[test]
    fn every_state_the_generator_names_is_a_vanilla_state() {
        let registry = BlockRegistry::vanilla();
        let known = |block: &Block| registry.find(&format!("minecraft:{}", block.name()), block.states()).is_some();
        let missing: Vec<String> = Block::ALL.iter().copied().chain(Block::templates()).filter(|block| !known(block)).map(|block| format!("{} [{}]", block.name(), block.states())).collect();
        assert!(missing.is_empty(), "the registry lacks {} states: {missing:?}", missing.len());
    }

    #[test]
    fn every_cell_reads_back_as_the_generated_block_and_biome() {
        let registry = BlockRegistry::vanilla();
        let generator = OverworldGenerator::new(20261003);
        let (shape, biomes) = (generator.chunk_blocks(0, 0), generator.chunk_biomes(0, 0));
        let dim = Dimension::overworld(registry.air_id());
        let chunk = generated_chunk(0, 0, &shape, &biomes, &BlockIds::of(registry), dim);
        for y in MIN_Y..MIN_Y + dim.height as i32 {
            for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))) {
                let (block, state) = (shape.block(x, y, z), registry.get(chunk.block(x as i32, y, z as i32)).unwrap());
                assert_eq!((state.name.strip_prefix("minecraft:"), state.properties), (Some(block.name()), block.states()), "at {x} {y} {z}");
                let second = block.second_layer().map_or(registry.air_id(), |layer| registry.find(&format!("minecraft:{}", layer.name()), layer.states()).unwrap());
                assert_eq!(chunk.liquid(x as i32, y, z as i32), second);
                assert_eq!(chunk.biome(x as i32, y, z as i32), Some(biomes.biome(x, y, z)));
            }
        }
    }
}
