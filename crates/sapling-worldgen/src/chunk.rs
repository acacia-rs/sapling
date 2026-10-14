//! A chunk column's blocks: 24 sections of 16x16x16, each shared until written (copy on write), so a
//! copy of a chunk costs 24 reference counts and owns only the sections it changes, and every
//! all-air section is one shared allocation. A read is one index into a section.

use std::sync::{Arc, OnceLock};

use crate::block::Block;

pub const MIN_Y: i32 = -64;
pub const HEIGHT: usize = 384;
const SECTION: usize = 16 * 16 * 16;
const SECTIONS: usize = HEIGHT / 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellKind {
    Air,
    Water,
    Lava,
    Solid,
}

/// A block in two bytes: a template palette index below 0x8000, else 0x8000 plus the variant's tag.
fn pack(block: Block) -> u16 {
    match block {
        Block::Template(index) => index,
        other => 0x8000 | (other.index() + 1) as u16,
    }
}

fn unpack(code: u16) -> Block {
    let raw = if code < 0x8000 { u32::from(code) << 16 } else { u32::from(code & 0x7FFF) };
    // Tag in the low half, `Template`'s index in the high half; a fieldless variant's high half is padding.
    unsafe { std::mem::transmute::<u32, Block>(raw) }
}

type Blocks = [u16; SECTION];

/// Shared sections are copied on their first write, owned ones written in place (no atomic per write);
/// `share` turns a finished chunk's owned sections into shared ones before caches copy it.
#[derive(Clone)]
enum Section {
    Shared(Arc<Blocks>),
    Owned(Box<Blocks>),
}

impl Section {
    fn blocks(&self) -> &Blocks {
        match self {
            Section::Shared(blocks) => blocks,
            Section::Owned(blocks) => blocks,
        }
    }

    fn blocks_mut(&mut self) -> &mut Blocks {
        if let Section::Shared(blocks) = self {
            *self = Section::Owned(Box::new(**blocks));
        }
        match self {
            Section::Owned(blocks) => blocks,
            Section::Shared(_) => unreachable!("made owned above"),
        }
    }
}

fn air_section() -> Section {
    static AIR: OnceLock<Arc<Blocks>> = OnceLock::new();
    Section::Shared(AIR.get_or_init(|| Arc::new([pack(Block::Air); SECTION])).clone())
}

/// One chunk column: `block(x, y, z)` with y from -64 to 319.
pub struct ChunkShape {
    /// `pack`ed blocks by section, bottom up.
    sections: [Section; SECTIONS],
    /// Each section's first block, so a read does not match on the variant.
    data: [*const u16; SECTIONS],
    /// Fluid cells the aquifer asked to tick (index, lava), in the order generation placed them:
    /// the chunk's tick queue when its decoration starts.
    fluid_ticks: Vec<(u32, bool)>,
}

// SAFETY: `data` points into the sections' own heap blocks, which never move and are written only
// through `&mut self`; the shape is as Send and Sync as its sections.
unsafe impl Send for ChunkShape {}
unsafe impl Sync for ChunkShape {}

impl Clone for ChunkShape {
    fn clone(&self) -> Self {
        Self::from_sections(self.sections.clone(), self.fluid_ticks.clone())
    }
}

impl ChunkShape {
    fn from_sections(sections: [Section; SECTIONS], fluid_ticks: Vec<(u32, bool)>) -> Self {
        let data = std::array::from_fn(|s| sections[s].blocks().as_ptr());
        Self { sections, data, fluid_ticks }
    }

    /// All air.
    pub(crate) fn empty() -> Self {
        Self::from_sections(std::array::from_fn(|_| air_section()), Vec::new())
    }

    pub(crate) fn schedule_fluid(&mut self, index: usize, lava: bool) {
        self.fluid_ticks.push((index as u32, lava));
    }

    pub(crate) fn fluid_ticks(&self) -> &[(u32, bool)] {
        &self.fluid_ticks
    }

    pub(crate) fn index(x: usize, y: i32, z: usize) -> usize {
        ((y - MIN_Y) as usize * 16 + z) * 16 + x
    }

    /// Its owned sections shared from now on, all-air ones as the one air section.
    pub(crate) fn share(&mut self) {
        let air = pack(Block::Air);
        for (section, data) in self.sections.iter_mut().zip(&mut self.data) {
            if let Section::Owned(blocks) = section {
                *section = if blocks.iter().all(|&code| code == air) { air_section() } else { Section::Shared(Arc::new(**blocks)) };
                *data = section.blocks().as_ptr();
            }
        }
    }

    fn code_at(&self, index: usize) -> u16 {
        assert!(index < SECTION * SECTIONS);
        // SAFETY: in bounds of the section `data` points into (checked above).
        unsafe { *self.data[index / SECTION].add(index % SECTION) }
    }

    /// Whether any block of the column satisfies `test`; a shared all-air section is tested once.
    pub(crate) fn any_block(&self, test: impl Fn(Block) -> bool) -> bool {
        let air = air_section();
        self.sections.iter().any(|section| match (section, &air) {
            (Section::Shared(blocks), Section::Shared(air)) if Arc::ptr_eq(blocks, air) => test(Block::Air),
            _ => section.blocks().iter().any(|&code| test(unpack(code))),
        })
    }

    pub(crate) fn block_at(&self, index: usize) -> Block {
        unpack(self.code_at(index))
    }

    /// Copies a shared section only when the block changes.
    pub(crate) fn set_at(&mut self, index: usize, block: Block) {
        let code = pack(block);
        let unchanged = self.code_at(index) == code;
        match &mut self.sections[index / SECTION] {
            Section::Owned(blocks) => blocks[index % SECTION] = code,
            Section::Shared(_) if unchanged => {}
            section => {
                let blocks = section.blocks_mut();
                blocks[index % SECTION] = code;
                self.data[index / SECTION] = blocks.as_ptr();
            }
        }
    }

    pub fn block(&self, x: usize, y: i32, z: usize) -> Block {
        self.block_at(Self::index(x, y, z))
    }

    pub fn set_block(&mut self, x: usize, y: i32, z: usize, block: Block) {
        self.set_at(Self::index(x, y, z), block);
    }

    /// Per column (`z * 16 + x`), one above the highest block that is not air.
    pub(crate) fn tops(&self) -> [i16; 256] {
        std::array::from_fn(|i| (MIN_Y..MIN_Y + HEIGHT as i32).rev().find(|&y| self.block(i % 16, y, i / 16) != Block::Air).map_or(MIN_Y, |y| y + 1) as i16)
    }

    pub fn kind(&self, x: usize, y: i32, z: usize) -> CellKind {
        match self.block(x, y, z) {
            Block::Air => CellKind::Air,
            block if block.is_lava() => CellKind::Lava,
            block if block.is_fluid() => CellKind::Water,
            _ => CellKind::Solid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ChunkShape, pack, unpack};
    use crate::block::Block;

    #[test]
    fn packed_blocks_round_trip() {
        let palette = crate::structure::template::palette().len();
        assert!(palette <= 0x8000 && Block::ALL.len() < 0x8000);
        for &block in Block::ALL.iter().chain(&[Block::Template(0), Block::Template(palette as u16 - 1)]) {
            assert_eq!(unpack(pack(block)), block);
        }
    }

    #[test]
    fn copies_own_only_written_sections() {
        let mut shape = ChunkShape::empty();
        shape.set_block(3, 70, 5, Block::Stone);
        let mut copy = shape.clone();
        copy.set_block(3, 70, 5, Block::Dirt);
        assert_eq!((shape.block(3, 70, 5), copy.block(3, 70, 5)), (Block::Stone, Block::Dirt));
        assert_eq!(copy.block(3, 71, 5), Block::Air);
    }
}
