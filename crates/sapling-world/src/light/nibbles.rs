//! Light as it is kept: a level per cell, two to a byte, per section and channel; a section at one
//! level throughout (open sky, solid ground, no lamp near) holds no array at all.

use acacia_world::Dimension;

const BYTES: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Channel {
    Sky,
    Block,
}

/// One channel of one section, cells in acacia-world's XZY order (`index`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Nibbles {
    Uniform(u8),
    Mixed(Box<[u8; BYTES]>),
}

impl Nibbles {
    pub fn get(&self, at: usize) -> u8 {
        match self {
            Self::Uniform(level) => *level,
            Self::Mixed(bytes) => bytes[at >> 1] >> ((at & 1) * 4) & 15,
        }
    }

    pub fn set(&mut self, at: usize, level: u8) {
        if let Self::Uniform(was) = *self {
            if was == level {
                return;
            }
            *self = Self::Mixed(Box::new([was | was << 4; BYTES]));
        }
        if let Self::Mixed(bytes) = self {
            let shift = (at & 1) * 4;
            bytes[at >> 1] = bytes[at >> 1] & !(15 << shift) | level << shift;
        }
    }

    pub fn uniform(&self) -> Option<u8> {
        match self {
            Self::Uniform(level) => Some(*level),
            Self::Mixed(_) => None,
        }
    }

    /// Lets go of the array of a section that came out all at one level.
    fn compact(&mut self) {
        if let Self::Mixed(bytes) = self {
            let first = bytes[0];
            if first >> 4 == first & 15 && bytes.iter().all(|byte| *byte == first) {
                *self = Self::Uniform(first & 15);
            }
        }
    }
}

/// A cell's place in its section: `(x << 8) | (z << 4) | y`, as `acacia_world::Chunk::copy_section`.
pub(crate) const fn index(x: i32, y: i32, z: i32) -> usize {
    ((x & 15) << 8 | (z & 15) << 4 | (y & 15)) as usize
}

/// Both channels of a column, bottom section first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColumnLight {
    min_y: i32,
    sky: Box<[Nibbles]>,
    block: Box<[Nibbles]>,
}

impl ColumnLight {
    /// A column of air: sky light everywhere where the dimension has a sky.
    pub fn open(dim: Dimension) -> Self {
        Self::filled(dim, if dim.sky { super::FULL_LIGHT } else { 0 })
    }

    pub fn dark(dim: Dimension) -> Self {
        Self::filled(dim, 0)
    }

    fn filled(dim: Dimension, sky: u8) -> Self {
        let sections = (dim.height / 16) as usize;
        Self { min_y: dim.min_y, sky: vec![Nibbles::Uniform(sky); sections].into(), block: vec![Nibbles::Uniform(0); sections].into() }
    }

    pub fn sections(&self) -> usize {
        self.sky.len()
    }

    fn slot(&self, y: i32) -> Option<usize> {
        usize::try_from((y - self.min_y) >> 4).ok().filter(|slot| *slot < self.sky.len())
    }

    pub fn section(&self, channel: Channel, slot: usize) -> &Nibbles {
        match channel {
            Channel::Sky => &self.sky[slot],
            Channel::Block => &self.block[slot],
        }
    }

    pub fn section_mut(&mut self, channel: Channel, slot: usize) -> &mut Nibbles {
        match channel {
            Channel::Sky => &mut self.sky[slot],
            Channel::Block => &mut self.block[slot],
        }
    }

    /// The level at a cell of the column (x and z taken within it); `None` outside its height.
    pub fn get(&self, channel: Channel, x: i32, y: i32, z: i32) -> Option<u8> {
        self.slot(y).map(|slot| self.section(channel, slot).get(index(x, y, z)))
    }

    pub fn set(&mut self, channel: Channel, x: i32, y: i32, z: i32, level: u8) {
        if let Some(slot) = self.slot(y) {
            self.section_mut(channel, slot).set(index(x, y, z), level);
        }
    }

    pub fn compact(&mut self) {
        self.sky.iter_mut().chain(self.block.iter_mut()).for_each(Nibbles::compact);
    }

    /// How many sections hold an array, both channels counted.
    #[cfg(test)]
    pub fn arrays(&self) -> usize {
        self.sky.iter().chain(self.block.iter()).filter(|nibbles| nibbles.uniform().is_none()).count()
    }
}
