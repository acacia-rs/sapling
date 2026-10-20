//! `LegacyStructureTemplate::placeInWorld` (141eeb8d0) with BDS's `LegacyStructureSettings`
//! (141ef1240): blocks in file order, structure blocks and jigsaws skipped, an integrity draw
//! per block, positions turned about the origin, writes clipped to the settings' box.

use super::mirror::mirror;
use super::rules::{Rule, apply};
use super::wood::rewood;
use super::{Kind, Template, soaked};
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::canvas::Canvas;
use crate::structure::{BoundingBox, trace};

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
    /// Off for pool elements: a jigsaw is placed as its final state, through the rules like any block.
    pub ignore_jigsaws: bool,
    /// A data structure block leaves air (pool elements: a villager's marker dug into a hill, village
    /// canon -1009,84,163); other pieces handle their markers themselves.
    pub marker_air: bool,
    /// Air placed below sea level becomes water (ocean ruins, shipwrecks).
    pub flood: bool,
    /// Blocks of this name are left out, before their integrity draw (template pieces skip air).
    pub ignore: Option<&'static str>,
    /// New wood per wood type for wooden blocks (shipwrecks).
    pub wood: Option<[Option<u8>; 6]>,
    /// Template z flipped before turning (ruined portals).
    pub mirror: bool,
    /// Where blocks may land; the canvas clips further to its chunk.
    pub bb: BoundingBox,
    /// Pool element processors, drawing from the integrity generator.
    pub rules: &'static [Rule],
    /// What the rules' position tests measure from (settings +0x20).
    pub pivot: (i32, i32, i32),
}

impl Placement {
    pub fn new(rotation: Rotation, bb: BoundingBox) -> Self {
        Self { rotation, integrity: 1.0, seed: 0, ignore_jigsaws: true, marker_air: false, flood: false, ignore: None, wood: None, mirror: false, bb, rules: &[], pivot: (0, 0, 0) }
    }

    /// The template positions BDS writes at `origin`, each with its block and template block
    /// index, in placement order.
    pub fn blocks<'a>(&'a self, template: &'a Template, origin: (i32, i32, i32)) -> impl Iterator<Item = ((i32, i32, i32), Block, usize)> + 'a {
        self.blocks_in(template, origin, &|_, _, _| Block::Air)
    }

    /// `blocks`, rule locations testing what `world` holds.
    pub fn blocks_in<'a>(
        &'a self,
        template: &'a Template,
        origin: (i32, i32, i32),
        world: &'a dyn Fn(i32, i32, i32) -> Block,
    ) -> impl Iterator<Item = ((i32, i32, i32), Block, usize)> + 'a {
        let mut random = (self.integrity < 1.0 || !self.rules.is_empty()).then(|| Mt19937::new(self.seed));
        template.blocks.iter().enumerate().filter_map(move |(index, b)| {
            if self.ignore.is_some_and(|name| b.state.block(Rotation::None).name() == name) {
                return None;
            }
            let state = match b.state.kind() {
                Kind::StructureBlock if !self.marker_air => return None,
                Kind::Jigsaw if self.ignore_jigsaws => return None,
                Kind::Jigsaw => template.jigsaws.iter().find(|j| j.block == index)?.final_state,
                Kind::Block | Kind::StructureBlock => b.state,
            };
            if let Some(random) = random.as_mut()
                && self.integrity < 1.0
                && self.integrity < random.next_float()
            {
                return None;
            }
            let pos = if self.mirror { (b.pos.0, b.pos.1, -b.pos.2) } else { b.pos };
            let (dx, dy, dz) = self.rotation.apply(pos);
            let at = (origin.0 + dx, origin.1 + dy, origin.2 + dz);
            if !self.bb.contains(at.0, at.1, at.2) {
                return None;
            }
            let mut block = state.block(self.rotation);
            if self.mirror {
                block = mirror(block, self.rotation);
            }
            if let Some(remap) = &self.wood {
                block = rewood(block, remap);
            }
            if let Some(random) = random.as_mut()
                && !self.rules.is_empty()
            {
                let from_pivot = (at.0 - self.pivot.0, at.1 - self.pivot.1, at.2 - self.pivot.2);
                block = apply(self.rules, block, world(at.0, at.1, at.2), from_pivot, random);
            }
            if self.flood && at.1 < SEA_LEVEL {
                block = if block == Block::Air { Block::Water } else { soaked(block) };
            }
            Some((at, block, index))
        })
    }

    /// Names the template block for the structure trace.
    pub fn label(&self, template: &Template, index: usize) {
        trace::label(|| {
            let b = &template.blocks[index];
            let raw = b.state.block(Rotation::None);
            format!("{} {:?} t{:?} {}[{}]", template.name, self.rotation, b.pos, raw.name(), raw.states())
        });
    }

    /// Writes the template; returns how many loot chests it placed (each drew a loot seed).
    pub fn place(&self, canvas: &mut Canvas, template: &Template, origin: (i32, i32, i32)) -> usize {
        let mut chests = 0;
        canvas.begin_template();
        for ((x, y, z), block, index) in self.blocks(template, origin) {
            self.label(template, index);
            canvas.set(x, y, z, block);
            chests += usize::from(template.loot_chests.contains(&index));
        }
        self.update_pass(canvas, template, origin);
        chests
    }

    /// Ends the placement `Canvas::begin_template` started: the template's box grown by one, in its own x, y, z order.
    pub fn update_pass(&self, canvas: &mut Canvas, template: &Template, origin: (i32, i32, i32)) {
        trace::label(|| format!("{} update pass", template.name));
        canvas.update_pass(pass_cells(self.rotation, self.mirror, template.size, origin), template.legacy);
    }
}

/// A template's cells and those around it as world positions, in the order of its own axes.
pub fn pass_cells(rotation: Rotation, mirror: bool, (sx, sy, sz): (i32, i32, i32), origin: (i32, i32, i32)) -> impl Iterator<Item = (i32, i32, i32)> {
    let local = (-1..=sx).flat_map(move |x| (-1..=sy).flat_map(move |y| (-1..=sz).map(move |z| (x, y, if mirror { -z } else { z }))));
    local.map(move |pos| {
        let (dx, dy, dz) = rotation.apply(pos);
        (origin.0 + dx, origin.1 + dy, origin.2 + dz)
    })
}
