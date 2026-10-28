//! Reading templates.bin (format in scripts/templates_to_rust.py): the block palette first, so
//! blocks resolve before the templates and the state table that point into it.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::{Jigsaw, Kind, State, Template, TemplateBlock};
use crate::block::Block;
use crate::block_entity::BlockEntity;

static DATA: &[u8] = include_bytes!("templates.bin");

/// A palette entry: name, states, waterlogged, flags.
pub type Entry = (&'static str, &'static str, bool, u8);

pub(super) struct Data {
    pub templates: HashMap<String, Template>,
    pub states: Vec<(Kind, [Block; 4])>,
}

/// Every Bedrock block templates and structure code place.
pub fn palette() -> &'static [Entry] {
    static PALETTE: OnceLock<Vec<Entry>> = OnceLock::new();
    PALETTE.get_or_init(|| {
        let mut r = Reader(&DATA[4..]);
        (0..r.u16())
            .map(|_| {
                let n = r.u8() as usize;
                (r.str(n), r.str_u16(), r.u8() != 0, r.u8())
            })
            .collect()
    })
}

pub(super) fn data() -> &'static Data {
    static PARSED: OnceLock<Data> = OnceLock::new();
    PARSED.get_or_init(parse)
}

/// A palette entry as a block: the `Block` variant with its name, states and water when there is
/// one (so terrain code recognises it), else `Block::Template`.
pub(super) fn palette_block(index: u16) -> Block {
    let (name, states, waterlogged, _) = palette()[index as usize];
    let same = |b: &&Block| b.name() == name && b.states() == states && b.is_waterlogged() == waterlogged;
    Block::ALL.iter().find(same).copied().unwrap_or(Block::Template(index))
}

struct Reader(&'static [u8]);

impl Reader {
    fn take(&mut self, n: usize) -> &'static [u8] {
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

    fn str(&mut self, n: usize) -> &'static str {
        std::str::from_utf8(self.take(n)).expect("utf-8 template string")
    }

    fn str_u16(&mut self) -> &'static str {
        let n = self.u16() as usize;
        self.str(n)
    }

    fn string(&mut self) -> String {
        self.short_str().to_string()
    }

    fn short_str(&mut self) -> &'static str {
        let n = self.u8() as usize;
        self.str(n)
    }

    /// A template block's entity (scripts/templates_to_rust.py `block_entity`); loot seeds are drawn at placement.
    fn entity(&mut self) -> BlockEntity {
        let some = |s: &'static str| (!s.is_empty()).then_some(s);
        match self.short_str() {
            "loot" => BlockEntity::Loot { table: self.short_str(), seed: 0 },
            "pot" => {
                let table = some(self.short_str());
                let sherds = [(); 4].map(|_| self.short_str());
                BlockEntity::Pot { table, seed: 0, sherds: some(sherds[0]).map(|_| sherds) }
            }
            "trial" => BlockEntity::TrialSpawner { normal: self.short_str(), ominous: self.short_str() },
            "vault" => BlockEntity::Vault { table: self.short_str(), key: self.short_str() },
            "spawner" => BlockEntity::Spawner(self.short_str()),
            kind => panic!("templates.bin: unknown block entity {kind}"),
        }
    }
}

fn parse() -> Data {
    assert_eq!(&DATA[..4], b"SPT1", "templates.bin: bad magic");
    let mut r = Reader(&DATA[4..]);
    for _ in 0..r.u16() {
        let n = r.u8() as usize;
        r.take(n);
        r.str_u16();
        r.take(2);
    }
    let count = r.u32();
    let mut templates = HashMap::with_capacity(count as usize);
    for _ in 0..count {
        let name = r.string();
        let size = (r.u16() as i32, r.u16() as i32, r.u16() as i32);
        let legacy = r.u8() != 0;
        let blocks = (0..r.u32())
            .map(|_| {
                let (x, y, z) = (r.u8() as i32, r.u16() as i32, r.u8() as i32);
                TemplateBlock { pos: (x, y, z), state: State(r.u16()) }
            })
            .collect();
        let (mut jigsaws, mut markers, mut loot_chests, mut ceiling_vines, mut entities) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for _ in 0..r.u16() {
            let block = r.u32() as usize;
            match r.u8() {
                0 => {
                    let (name, target, pool, joint, orientation) = (r.string(), r.string(), r.string(), r.string(), r.string());
                    let final_state = State(r.u16());
                    let (selection_priority, placement_priority) = (r.u32() as i32, r.u32() as i32);
                    jigsaws.push(Jigsaw { block, name, target, pool, joint, orientation, final_state, selection_priority, placement_priority });
                }
                1 => markers.push((block, r.string())),
                2 => loot_chests.push(block),
                3 => ceiling_vines.push(block),
                _ => entities.push((block, r.entity())),
            }
        }
        templates.insert(name.clone(), Template { name, size, legacy, blocks, jigsaws, markers, loot_chests, ceiling_vines, entities });
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
