//! `MultifaceSpreader` (spread 1465e8f20, toward all 1465e9af0, toward random 1465e9360) and
//! `MultifaceBlock::getStateForPlacement` (148b68b30). Faces are FacingIDs: down, up, north,
//! south, west, east.

use super::view::View;
use crate::block::Block;

const FACING: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

/// The order BDS walks faces in `spreadAll` and its toward-all loops.
pub const SPREAD_ORDER: [usize; 6] = [1, 0, 2, 3, 4, 5];

pub fn step([x, y, z]: [i32; 3], face: usize) -> [i32; 3] {
    let [dx, dy, dz] = FACING[face];
    [x + dx, y + dy, z + dz]
}

pub fn opposite(face: usize) -> usize {
    face ^ 1
}

pub fn has_face(block: Block, face: usize) -> bool {
    block.multiface().is_some_and(|(_, bits)| bits & Block::FACE_BITS[face] != 0)
}

/// `getStateForPlacement`: `existing` with `face` added (a fresh `first` when it is another
/// block), or `existing` itself when the face is there already or has nothing to hang on.
pub fn placement(view: &View, first: Block, at: [i32; 3], existing: Block, face: usize) -> Block {
    let own = existing.multiface().filter(|&(family, _)| family == first).map_or(0, |(_, bits)| bits);
    let [x, y, z] = step(at, face);
    if own & Block::FACE_BITS[face] != 0 || !view.block(x, y, z).supports_multiface(opposite(face)) {
        return existing;
    }
    Block::with_faces(first, own | Block::FACE_BITS[face])
}

/// A spreader's config: glow lichen's default one, or sculk vein's (all spread types, or the
/// same cell only).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Config {
    Lichen,
    Vein,
    VeinSameSpace,
}

impl Config {
    pub fn first(self) -> Block {
        if self == Config::Lichen { Block::GlowLichen1 } else { Block::SculkVein1 }
    }

    /// Spread types in order: 0 same cell, 1 same plane, 2 around the edge.
    fn kinds(self) -> &'static [u8] {
        if self == Config::VeinSameSpace { &[0] } else { &[0, 1, 2] }
    }

    fn can_spread_from(self, state: Block, from: usize) -> bool {
        self == Config::Lichen || !is_vein(state) || has_face(state, from)
    }

    fn other_valid_as_source(self, state: Block) -> bool {
        self != Config::Lichen && !is_vein(state)
    }

    /// `canSpreadInto`: lichen's `stateCanBeReplaced` (1465e9f50); vein's (148b4a9d0) also
    /// refuses a face on sculk or a catalyst and takes anything replaceable.
    fn can_spread_into(self, view: &View, at: [i32; 3], face: usize) -> bool {
        let state = view.block(at[0], at[1], at[2]);
        if self != Config::Lichen {
            let [x, y, z] = step(at, face);
            if matches!(view.block(x, y, z), Block::Sculk | Block::SculkCatalyst) {
                return false;
            }
            if vein_replaceable(state) {
                return true;
            }
        }
        state == Block::Air || state == Block::Water || state.multiface().is_some_and(|(family, _)| family == self.first())
    }
}

pub fn is_vein(block: Block) -> bool {
    block.multiface().is_some_and(|(family, _)| family == Block::SculkVein1)
}

/// 148b41420: a water source, or a replaceable non-fluid.
pub fn vein_replaceable(block: Block) -> bool {
    // Snow layers are replaceable too: the city's ice box floor (one-worker run, veins set into them).
    block == Block::Water || block == Block::Air || super::tree::common::is_replaceable_plant(block) || block.multiface().is_some() || block.name() == "snow_layer"
}

/// `getSpreadFromFaceTowardDirection`: where a `state` at `at` on face `from` spreads toward
/// `to`, as the cell, its face and the block it would become.
pub fn spread_target(config: Config, view: &View, state: Block, at: [i32; 3], from: usize, to: usize) -> Option<([i32; 3], usize, Block)> {
    if !config.can_spread_from(state, from) || to == from || to == opposite(from) {
        return None;
    }
    if !(config.other_valid_as_source(state) || has_face(state, from) && !has_face(state, to)) {
        return None;
    }
    config.kinds().iter().find_map(|&kind| {
        let (cell, face) = match kind {
            0 => (at, to),
            1 => (step(at, to), from),
            _ => (step(step(at, from), to), opposite(to)),
        };
        if !config.can_spread_into(view, cell, face) {
            return None;
        }
        let existing = view.block(cell[0], cell[1], cell[2]);
        let placed = placement(view, config.first(), cell, existing, face);
        (placed != existing).then_some((cell, face, placed))
    })
}

/// `spreadFromFaceTowardAllDirections` over `faces` in order (`spreadAll` when all six): the
/// blocks placed.
pub fn spread_all(config: Config, view: &mut View, state: Block, at: [i32; 3], faces: &[usize]) -> usize {
    let mut placed = 0;
    for &from in faces {
        for to in SPREAD_ORDER {
            if let Some((cell, _, block)) = spread_target(config, view, state, at, from, to) {
                view.set_block(cell[0], cell[1], cell[2], block);
                placed += 1;
            }
        }
    }
    placed
}
