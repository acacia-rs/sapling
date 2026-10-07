//! BDS's structure templates: the Java `.nbt` files of the vanilla behaviour packs, transcribed
//! by scripts/templates_to_rust.py (format in its docstring), with each Java state's Bedrock block
//! per rotation as BDS's loader (`LegacyStructureTemplate` load 141eedcf0, converters 141f75940 and
//! 141f71f20) and rotation (141f70580) make it.

mod blocks;
mod markers;
mod place;
mod wood;

use std::collections::HashMap;
use std::sync::OnceLock;

pub use blocks::TEMPLATE_BLOCKS;
#[allow(unused_imports)]
pub use place::{Placement, Rotation};

use crate::block::Block;

static DATA: &[u8] = include_bytes!("templates.bin");

/// `TEMPLATE_BLOCKS` flags.
pub const FULL_CUBE: u8 = 1;
pub const COLLIDES: u8 = 2;
pub const WATERLOGGABLE: u8 = 4;

/// A block's `TEMPLATE_BLOCKS` flags (0 for blocks templates never place).
pub fn flags(block: Block) -> u8 {
    static INDEX: OnceLock<HashMap<(&str, &str, bool), u8>> = OnceLock::new();
    let index = INDEX.get_or_init(|| TEMPLATE_BLOCKS.iter().map(|&(n, s, w, f)| ((n, s, w), f)).collect());
    index.get(&(block.name(), block.states(), block.is_waterlogged())).copied().unwrap_or(0)
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
    pub final_state: State,
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

struct Data {
    templates: HashMap<String, Template>,
    states: Vec<(Kind, [Block; 4])>,
}

fn data() -> &'static Data {
    static DATA_PARSED: OnceLock<Data> = OnceLock::new();
    DATA_PARSED.get_or_init(|| parse(DATA))
}

/// A block structure code places itself, by its saved name and states; panics on one that is
/// neither a `Block` variant nor in `TEMPLATE_BLOCKS` (scripts/templates_to_rust.py `EXTRA`).
pub fn named(name: &str, states: &str, waterlogged: bool) -> Block {
    static INDEX: OnceLock<HashMap<(&str, &str, bool), u16>> = OnceLock::new();
    let index = INDEX.get_or_init(|| TEMPLATE_BLOCKS.iter().enumerate().map(|(i, &(n, s, w, _))| ((n, s, w), i as u16)).collect());
    let found = index.get(&(name, states, waterlogged)).unwrap_or_else(|| panic!("no template block {name}[{states}]"));
    palette_block(*found)
}

/// A `TEMPLATE_BLOCKS` entry as a block: the `Block` variant with its name, states and water when
/// there is one (so terrain code recognises it), else `Block::Template`.
fn palette_block(index: u16) -> Block {
    let (name, states, waterlogged, _) = TEMPLATE_BLOCKS[index as usize];
    let same = |b: &&Block| b.name() == name && b.states() == states && b.is_waterlogged() == waterlogged;
    Block::ALL.iter().find(same).copied().unwrap_or(Block::Template(index))
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn take(&mut self, n: usize) -> &[u8] {
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        head
    }

    fn u8(&mut self) -> u8 {
        self.take(1)[0]
    }

    fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.take(2).try_into().unwrap())
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take(4).try_into().unwrap())
    }

    fn string(&mut self) -> String {
        let n = self.u8() as usize;
        String::from_utf8(self.take(n).to_vec()).expect("utf-8 template string")
    }
}

fn parse(data: &[u8]) -> Data {
    let mut r = Reader(data);
    assert_eq!(r.take(4), b"SPT1", "templates.bin: bad magic");
    let count = r.u32();
    let mut templates = HashMap::with_capacity(count as usize);
    for _ in 0..count {
        let name = r.string();
        let size = (r.u16() as i32, r.u16() as i32, r.u16() as i32);
        let blocks = (0..r.u32())
            .map(|_| {
                let (x, y, z) = (r.u8() as i32, r.u16() as i32, r.u8() as i32);
                TemplateBlock { pos: (x, y, z), state: State(r.u16()) }
            })
            .collect();
        let (mut jigsaws, mut markers, mut loot_chests) = (Vec::new(), Vec::new(), Vec::new());
        for _ in 0..r.u16() {
            let block = r.u32() as usize;
            match r.u8() {
                0 => {
                    let (name, target, pool, joint) = (r.string(), r.string(), r.string(), r.string());
                    jigsaws.push(Jigsaw { block, name, target, pool, joint, final_state: State(r.u16()) });
                }
                1 => markers.push((block, r.string())),
                _ => loot_chests.push(block),
            }
        }
        templates.insert(name.clone(), Template { name, size, blocks, jigsaws, markers, loot_chests });
    }
    let mut blocks: HashMap<u16, Block> = HashMap::new();
    let states = (0..r.u16())
        .map(|_| {
            let kind = [Kind::Block, Kind::Jigsaw, Kind::StructureBlock][r.u8() as usize];
            (kind, [(); 4].map(|_| *blocks.entry(r.u16()).or_insert_with_key(|&i| palette_block(i))))
        })
        .collect();
    Data { templates, states }
}
