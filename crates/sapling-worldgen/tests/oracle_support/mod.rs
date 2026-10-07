//! Reading oracle worlds and sorting every cell where sapling and BDS disagree into the stage
//! that decides it, or into the later stages (features, structures, liquid flow) sapling does
//! not run yet.
#![allow(dead_code)]

mod carver;
mod features;
mod judge;
mod tally;

use std::path::PathBuf;

use acacia_world::Chunk;
use sapling_leveldb::LevelDbWorld;
use sapling_worldgen::{Block, CellKind};

// Shared by the oracle test and the examples; each uses a different part.
#[allow(unused_imports)]
pub use carver::{CarverTally, Ours, near_feature};
#[allow(unused_imports)]
pub use features::FeatureTally;
#[allow(unused_imports)]
pub use judge::{Context, column_reshaped};
#[allow(unused_imports)]
pub use tally::{BiomeTally, Tally, TopTally};

pub struct Region {
    pub name: &'static str,
    x0: i32,
    z0: i32,
    x1: i32,
    z1: i32,
}

impl Region {
    pub fn new(name: &'static str, x0: i32, z0: i32, x1: i32, z1: i32) -> Self {
        Self { name, x0, z0, x1, z1 }
    }

    pub fn chunks(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (self.x0..=self.x1).flat_map(move |x| (self.z0..=self.z1).map(move |z| (x, z)))
    }
}

/// The world and its `-run2` twin, or None when the oracle worlds are absent.
pub fn open(world: &str) -> Option<(LevelDbWorld, LevelDbWorld)> {
    let root = std::env::var_os("SAPLING_ORACLE_WORLDS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../worlds/oracle"));
    let (a, b) = (root.join(world), root.join(format!("{world}-run2")));
    if !a.exists() || !b.exists() {
        return None;
    }
    Some((LevelDbWorld::open_in_memory(&a).unwrap(), LevelDbWorld::open_in_memory(&b).unwrap()))
}

/// Blocks the terrain stages (shape, surface, ore veins, deepslate) place; any other solid block
/// is decoration (features, structures, carvers' side effects).
pub const TERRAIN_BLOCKS: &[&str] = &[
    "stone", "deepslate", "granite", "diorite", "andesite", "tuff", "dirt", "grass_block", "coarse_dirt", "podzol", "mycelium",
    "sand", "red_sand", "sandstone", "red_sandstone", "gravel", "clay", "mud", "snow", "powder_snow", "ice", "packed_ice",
    "blue_ice", "bedrock", "hardened_clay", "white_terracotta", "orange_terracotta", "yellow_terracotta",
    "brown_terracotta", "red_terracotta", "light_gray_terracotta", "copper_ore", "deepslate_copper_ore", "raw_copper_block",
    "iron_ore", "deepslate_iron_ore", "raw_iron_block", "coal_ore", "deepslate_coal_ore", "gold_ore", "deepslate_gold_ore",
    "redstone_ore", "deepslate_redstone_ore", "lapis_ore", "deepslate_lapis_ore", "diamond_ore", "deepslate_diamond_ore",
    "emerald_ore", "deepslate_emerald_ore", "infested_stone", "infested_deepslate",
];

/// A saved cell: block name without `minecraft:`, its states, and the name of a liquid in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BdsCell {
    pub block: String,
    pub states: String,
    pub liquid: Option<String>,
}

pub fn bds_cell(world: &LevelDbWorld, chunk: &Chunk, x: i32, y: i32, z: i32) -> BdsCell {
    let registry = world.registry();
    let state = |id| registry.get(id).map_or(("?".to_string(), String::new()), |s| (s.name.trim_start_matches("minecraft:").to_string(), s.properties.to_string()));
    let (block, states) = state(chunk.block(x, y, z));
    let liquid = chunk.liquid(x, y, z);
    let liquid = (liquid != registry.air_id()).then(|| state(liquid).0);
    BdsCell { block, states, liquid }
}

impl BdsCell {
    /// The same block and states, with water in the second layer exactly when the block is waterlogged.
    pub fn is(&self, block: Block) -> bool {
        let liquid = block.is_waterlogged().then_some("water");
        self.block == block.name() && self.states == block.states() && self.liquid.as_deref() == liquid
    }

    pub fn is_empty(&self) -> bool {
        matches!(self.block.as_str(), "air" | "cave_air" | "void_air")
    }

    pub fn is_fluid(&self) -> bool {
        matches!(self.block.as_str(), "water" | "flowing_water" | "lava" | "flowing_lava")
    }

    /// A block the terrain stages can place (whatever its states); trees' solid blocks are not.
    fn terrain(&self) -> Option<Block> {
        let tree = |b: Block| b.is_wood() || matches!(b, Block::MossyCobblestone | Block::MangroveRoots | Block::MuddyMangroveRoots | Block::BeeNest);
        Block::from_name(&self.block).filter(|&b| b.is_solid() && !tree(b))
    }

    /// Air, water, lava or solid; a waterlogged block counts as water.
    pub fn kind(&self) -> CellKind {
        match self.block.as_str() {
            "air" | "cave_air" | "void_air" => CellKind::Air,
            "water" | "flowing_water" => CellKind::Water,
            "lava" | "flowing_lava" => CellKind::Lava,
            _ if matches!(self.liquid.as_deref(), Some("water" | "flowing_water")) => CellKind::Water,
            _ => CellKind::Solid,
        }
    }

    /// Water, or a solid block the terrain stages place.
    pub fn is_terrain(&self) -> bool {
        match self.kind() {
            CellKind::Water => true,
            CellKind::Solid => TERRAIN_BLOCKS.contains(&self.block.as_str()),
            _ => false,
        }
    }
}
