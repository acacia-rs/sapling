//! `LegacyStructureTemplate::placeInWorld` (141eeb8d0) with BDS's `LegacyStructureSettings`
//! (141ef1240): blocks in file order, structure blocks and jigsaws skipped, an integrity draw
//! per block, positions turned about the origin, writes clipped to the settings' box.

use super::{Kind, Template};
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::BoundingBox;
use crate::structure::canvas::Canvas;

const SEA_LEVEL: i32 = 63;

/// Clockwise quarter turns, as BDS's `Rotation` enum counts them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rotation {
    None = 0,
    Clockwise90 = 1,
    Clockwise180 = 2,
    CounterClockwise90 = 3,
}

impl Rotation {
    pub const ALL: [Rotation; 4] = [Rotation::None, Rotation::Clockwise90, Rotation::Clockwise180, Rotation::CounterClockwise90];

    pub fn from_index(index: i32) -> Self {
        Self::ALL[index.rem_euclid(4) as usize]
    }

    /// A template offset turned about the origin.
    pub fn apply(self, (x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
        match self {
            Rotation::None => (x, y, z),
            Rotation::Clockwise90 => (-z, y, x),
            Rotation::Clockwise180 => (-x, y, -z),
            Rotation::CounterClockwise90 => (z, y, -x),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Placement {
    pub rotation: Rotation,
    pub integrity: f32,
    /// Seeds the integrity draws.
    pub seed: u32,
    pub ignore_jigsaws: bool,
    /// Air placed below sea level becomes water (ocean ruins, shipwrecks).
    pub flood: bool,
    /// Where blocks may land; the canvas clips further to its chunk.
    pub bb: BoundingBox,
}

impl Placement {
    pub fn new(rotation: Rotation, bb: BoundingBox) -> Self {
        Self { rotation, integrity: 1.0, seed: 0, ignore_jigsaws: true, flood: false, bb }
    }

    /// The template positions BDS writes at `origin`, each with its block, in placement order.
    pub fn blocks<'a>(&'a self, template: &'a Template, origin: (i32, i32, i32)) -> impl Iterator<Item = ((i32, i32, i32), Block)> + 'a {
        let mut random = (self.integrity < 1.0).then(|| Mt19937::new(self.seed));
        template.blocks.iter().filter_map(move |b| {
            match b.state.kind() {
                Kind::StructureBlock => return None,
                Kind::Jigsaw if self.ignore_jigsaws => return None,
                _ => {}
            }
            if let Some(random) = random.as_mut()
                && self.integrity < random.next_float()
            {
                return None;
            }
            let (dx, dy, dz) = self.rotation.apply(b.pos);
            let at = (origin.0 + dx, origin.1 + dy, origin.2 + dz);
            if !self.bb.contains(at.0, at.1, at.2) {
                return None;
            }
            let block = b.state.block(self.rotation);
            let block = if self.flood && block == Block::Air && at.1 < SEA_LEVEL { Block::Water } else { block };
            Some((at, block))
        })
    }

    pub fn place(&self, canvas: &mut Canvas, template: &Template, origin: (i32, i32, i32)) {
        for ((x, y, z), block) in self.blocks(template, origin) {
            canvas.set(x, y, z, block);
        }
    }
}
