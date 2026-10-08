//! BDS's structure templates: the Java `.nbt` files of the vanilla behaviour packs, transcribed
//! by scripts/templates_to_rust.py, with each Java state's Bedrock block per rotation as BDS's
//! loader (`LegacyStructureTemplate` load 141eedcf0, converters 141f75940 and 141f71f20) and
//! rotation (141f70580) make it.

mod connect;
mod java;
mod load;
mod markers;
mod mirror;
mod place;
mod rules;
mod wall;
mod wood;

use std::collections::HashMap;
use std::sync::OnceLock;

pub use connect::rejoin;
pub use java::{Capped, JavaPlacement};
pub use load::palette;
pub(crate) use markers::hash as marker_hash;
#[allow(unused_imports)]
pub use place::{Placement, Rotation};
pub use rules::{Predicate, Rule};

use crate::block::Block;
use load::{data, palette_block};

/// Palette flags.
pub const FULL_CUBE: u8 = 1;
pub const COLLIDES: u8 = 2;
pub const WATERLOGGABLE: u8 = 4;

/// A block's palette flags (0 for blocks templates never place).
pub fn flags(block: Block) -> u8 {
    static INDEX: OnceLock<HashMap<(&str, &str, bool), u8>> = OnceLock::new();
    let index = INDEX.get_or_init(|| palette().iter().map(|&(n, s, w, f)| ((n, s, w), f)).collect());
    index.get(&(block.name(), block.states(), block.is_waterlogged())).copied().unwrap_or(0)
}

/// A block structure code places itself, by its saved name and states; panics on one the palette
/// lacks (scripts/templates_palette.py says what it holds).
pub fn named(name: &str, states: &str, waterlogged: bool) -> Block {
    try_named(name, states, waterlogged).unwrap_or_else(|| panic!("no template block {name}[{states}]"))
}

pub fn try_named(name: &str, states: &str, waterlogged: bool) -> Option<Block> {
    static INDEX: OnceLock<HashMap<(&str, &str, bool), u16>> = OnceLock::new();
    let index = INDEX.get_or_init(|| palette().iter().enumerate().map(|(i, &(n, s, w, _))| ((n, s, w), i as u16)).collect());
    index.get(&(name, states, waterlogged)).map(|&i| palette_block(i))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Block,
    Jigsaw,
    StructureBlock,
}

/// A template state: an index into the state table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct State(u16);

impl State {
    pub(super) fn kind(self) -> Kind {
        data().states[self.0 as usize].0
    }

    /// The block this state places when the template is turned `rotation` clockwise.
    pub fn block(self, rotation: Rotation) -> Block {
        data().states[self.0 as usize].1[rotation as usize]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TemplateBlock {
    pub pos: (i32, i32, i32),
    pub state: State,
}

#[derive(Debug, Clone)]
pub struct Jigsaw {
    pub block: usize,
    pub name: String,
    pub target: String,
    pub pool: String,
    pub joint: String,
    /// Java's `orientation` state, front then top (e.g. "down_east").
    pub orientation: String,
    pub final_state: State,
    pub selection_priority: i32,
    pub placement_priority: i32,
}

#[derive(Debug)]
pub struct Template {
    pub name: String,
    pub size: (i32, i32, i32),
    /// In file order, which is the order BDS places them in.
    pub blocks: Vec<TemplateBlock>,
    pub jigsaws: Vec<Jigsaw>,
    /// Data structure blocks: block index and metadata.
    pub markers: Vec<(usize, String)>,
    /// Chests with block entity data: placing one draws a loot table seed from the piece's
    /// generator (placeInWorld, block actor type 2 without `LootTableSeed`).
    pub loot_chests: Vec<usize>,
}

impl Template {
    /// The vanilla template at `path` (e.g. "igloo/igloo_bottom"); panics on an unknown one.
    pub fn get(path: &str) -> &'static Template {
        Self::find(path).unwrap_or_else(|| panic!("no structure template {path}"))
    }

    pub fn find(path: &str) -> Option<&'static Template> {
        data().templates.get(path)
    }
}
