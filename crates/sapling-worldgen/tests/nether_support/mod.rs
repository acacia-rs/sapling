//! Reading a Nether canon: the BDS world, its `-run2` twin and the chunk buffers a debugger run dumped as
//! `loadChunk` committed them (scripts/data/dbg_nether_buffers.json), and our stages of a chunk.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sapling_worldgen::{Block, NetherGenerator};

pub const CELLS: usize = 16 * 16 * 128;

/// `<world>-buffers`: `block_ptrs.tsv` and one `ptr-nether-<x>_<z>-0.bin` of `Block*` per chunk, laid out `[x][z][y]`.
pub struct Buffers {
    names: HashMap<u64, u16>,
    pub kinds: Vec<String>,
    dir: PathBuf,
}

impl Buffers {
    pub fn open(dir: &Path) -> Option<Self> {
        let table = std::fs::read_to_string(dir.join("block_ptrs.tsv")).ok()?;
        let (mut names, mut kinds, mut index) = (HashMap::new(), Vec::new(), HashMap::new());
        for line in table.lines() {
            let mut parts = line.split('\t');
            let pointer = u64::from_str_radix(parts.next()?, 16).ok()?;
            let name = parts.next()?.trim_start_matches("minecraft:").to_string();
            let id = *index.entry(name.clone()).or_insert_with(|| {
                kinds.push(name);
                (kinds.len() - 1) as u16
            });
            names.insert(pointer, id);
        }
        Some(Self { names, kinds, dir: dir.to_path_buf() })
    }

    /// The chunk's block names by `[x][z][y]` index, if the run dumped it.
    pub fn chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<Vec<&str>> {
        let raw = std::fs::read(self.dir.join(format!("ptr-nether-{chunk_x}_{chunk_z}-0.bin"))).ok()?;
        assert_eq!(raw.len(), CELLS * 8);
        Some(raw.chunks_exact(8).map(|cell| self.names.get(&u64::from_le_bytes(cell.try_into().unwrap())).map_or("?", |&id| self.kinds[id as usize].as_str())).collect())
    }

    /// Every chunk the run dumped.
    pub fn chunks(&self) -> Vec<(i32, i32)> {
        let mut chunks: Vec<(i32, i32)> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let name = entry.ok()?.file_name().into_string().ok()?;
                let (x, z) = name.strip_prefix("ptr-nether-")?.strip_suffix("-0.bin")?.split_once('_')?;
                Some((x.parse().ok()?, z.parse().ok()?))
            })
            .collect();
        chunks.sort();
        chunks
    }
}

/// `[x][z][y]` buffer index of a chunk-local cell.
pub fn index(x: usize, y: i32, z: usize) -> usize {
    (x * 16 + z) * 128 + y as usize
}

/// Our chunk after each stage of `loadChunk`, for naming the stage a cell went wrong in.
pub struct Stages {
    pub shape: sapling_worldgen::ChunkShape,
    pub bedrock: sapling_worldgen::ChunkShape,
    pub surfaced: sapling_worldgen::ChunkShape,
    pub carved: sapling_worldgen::ChunkShape,
}

impl Stages {
    pub fn new(generator: &NetherGenerator, chunk_x: i32, chunk_z: i32) -> Self {
        Self {
            shape: generator.chunk_shape(chunk_x, chunk_z),
            bedrock: generator.bedrock_blocks(chunk_x, chunk_z),
            surfaced: generator.surfaced_blocks(chunk_x, chunk_z),
            carved: generator.carved_blocks(chunk_x, chunk_z),
        }
    }

    /// The stage a cell where ours differs from BDS's `loaded` block belongs to: ours carved it, or BDS emptied a
    /// cell our density filled (a carver miss: the density alone is checked by `shape`), or ours is open where BDS
    /// is solid (the density), or both are solid (the bedrock or a surface builder).
    pub fn stage_of(&self, x: usize, y: i32, z: usize, loaded: &str) -> &'static str {
        let at = |shape: &sapling_worldgen::ChunkShape| shape.block(x, y, z);
        let open = |name: &str| matches!(name, "air" | "lava" | "water");
        if at(&self.carved) != at(&self.surfaced) || (open(loaded) && at(&self.shape) == Block::Stone) {
            "carvers"
        } else if open(at(&self.carved).name()) != open(loaded) {
            "shape"
        } else if at(&self.carved) == Block::Bedrock || loaded == "bedrock" {
            "bedrock"
        } else {
            "surface"
        }
    }

    pub fn block(&self, x: usize, y: i32, z: usize) -> Block {
        self.carved.block(x, y, z)
    }
}

/// Blocks only the structure stage places in the Nether (fortresses, bastions, ruined portals), for splitting the
/// later stages' cells.
pub fn is_structure_block(name: &str) -> bool {
    ["nether_brick", "polished_blackstone", "gilded_blackstone", "chiseled_"].iter().any(|part| name.contains(part))
        || matches!(name, "chest" | "mob_spawner" | "gold_block" | "obsidian" | "crying_obsidian" | "nether_wart" | "lodestone" | "iron_chain" | "lantern" | "soul_lantern")
}
