//! Reading oracle worlds and sorting every cell where sapling and BDS disagree into the stage
//! that decides it, or into the later stages (features, structures, liquid flow) sapling does
//! not run yet.
#![allow(dead_code)]

mod carver;
mod dump;
mod features;
mod floor;
mod judge;
mod stages;
mod structures;
mod tally;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use acacia_world::Chunk;
use sapling_leveldb::LevelDbWorld;
use sapling_worldgen::{Block, CellKind, StructureStep};

// Shared by the oracle test and the examples; each uses a different part.
#[allow(unused_imports)]
pub use carver::{CarverTally, Ours, near_feature};
#[allow(unused_imports)]
pub use dump::Dump;
#[allow(unused_imports)]
pub use features::FeatureTally;
#[allow(unused_imports)]
pub use floor::Floor;
#[allow(unused_imports)]
pub use judge::{Context, column_reshaped};
#[allow(unused_imports)]
pub use stages::Stages;
#[allow(unused_imports)]
pub use structures::{PORTED, StageDiff, StructureTally, attribute, stage_diff};
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

/// `SAPLING_ORACLE_ORDER=<file>`: decorate in a one-worker BDS run's order (`x z [draws]` per line,
/// scripts/dbg_loads.py); `SAPLING_ORACLE_THREAD_SEED=<seed>`: also model that run's thread generator
/// from the lines' draw counts. `SAPLING_ORACLE_STRUCTURE_ORDER=<file>`: run the structure stage in that
/// run's order (`x z` per post-processed chunk and `load x z` per loaded one, `dbg_loads.py --structure-order`).
/// `SAPLING_ORACLE_FIXED_FACES`: the canonical order without its thread generator, a diagnostic.
pub fn replay(generator: &mut sapling_worldgen::OverworldGenerator) -> Option<Vec<(i32, i32)>> {
    if std::env::var_os("SAPLING_ORACLE_FIXED_FACES").is_some() {
        generator.fix_face_choices();
    }
    let numbers = |line: &str| -> Vec<i64> { line.split(' ').map(|v| v.parse().expect("an integer")).collect() };
    let read = |path: String| -> Vec<Vec<i64>> { std::fs::read_to_string(path).expect("order file").lines().map(numbers).collect() };
    if let Ok(path) = std::env::var("SAPLING_ORACLE_STRUCTURE_ORDER") {
        let step = |line: &str| match line.strip_prefix("load ") {
            Some(chunk) => StructureStep::Load((numbers(chunk)[0] as i32, numbers(chunk)[1] as i32)),
            None => StructureStep::Post((numbers(line)[0] as i32, numbers(line)[1] as i32)),
        };
        let steps: Vec<StructureStep> = std::fs::read_to_string(path).expect("structure order file").lines().map(step).collect();
        generator.set_structure_order(&steps);
    }
    let rows = read(std::env::var("SAPLING_ORACLE_ORDER").ok()?);
    let order: Vec<(i32, i32)> = rows.iter().map(|row| (row[0] as i32, row[1] as i32)).collect();
    let spent: Vec<Option<u32>> = rows.iter().map(|row| row.get(2).map(|&draws| draws as u32)).collect();
    let seed = std::env::var("SAPLING_ORACLE_THREAD_SEED").ok().map(|seed| seed.parse::<u32>().expect("a u32 seed"));
    generator.set_decoration_order(&order, seed.map(|seed| (seed, spent.as_slice())));
    Some(order)
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
    "emerald_ore", "deepslate_emerald_ore", "infested_stone", "infested_deepslate", "sulfur", "cinnabar",
];

/// A saved cell: block name without `minecraft:`, its states, and the name of a liquid in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BdsCell {
    pub block: String,
    pub states: String,
    pub liquid: Option<String>,
}

pub fn bds_cell(world: &LevelDbWorld, chunk: &Chunk, x: i32, y: i32, z: i32) -> BdsCell {
    thread_local! {
        /// Name and states by registry and id: formatting them for every cell was a third of a run.
        static STATES: RefCell<HashMap<(usize, u32), (String, String)>> = RefCell::default();
    }
    let registry = world.registry();
    let format = |id| registry.get(id).map_or(("?".to_string(), String::new()), |s| (s.name.trim_start_matches("minecraft:").to_string(), s.properties.to_string()));
    let state = |id: u32| STATES.with_borrow_mut(|states| states.entry((Arc::as_ptr(registry) as usize, id)).or_insert_with(|| format(id)).clone());
    let (block, states) = state(chunk.block(x, y, z));
    let liquid = chunk.liquid(x, y, z);
    let liquid = (liquid != registry.air_id()).then(|| state(liquid).0);
    BdsCell { block, states, liquid }
}

impl BdsCell {
    /// The same block and states, and the same second layer (a waterlogged block's water, a snow layer's plant).
    pub fn is(&self, block: Block) -> bool {
        let liquid = block.second_layer().map(Block::name);
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
        let tree = |b: Block| b.is_wood() || matches!(b, Block::MossyCobblestone | Block::MangroveRoots | Block::MuddyMangroveRoots | Block::BeeNest | Block::BrownMushroomBlock1 | Block::RedMushroomBlock1 | Block::MushroomStem);
        // Liquids harden to these, but monster rooms and structures place far more of them.
        let hardened = |b: Block| matches!(b, Block::Cobblestone | Block::Obsidian);
        Block::from_name(&self.block).filter(|&b| b.is_solid() && !tree(b) && !hardened(b) && !b.is_structure())
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
