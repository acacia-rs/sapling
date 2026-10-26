//! The start of a v2 jigsaw structure (`br::worldgen::JigsawAssembler::construct` 144ce64d0, from
//! the structure's generate 144ce7f40): with the start chunk's large-feature `java.util.Random`, the
//! start height, a rotation and a start element, its bottom one below the start height (plus the
//! surface at its middle when projected), then the queued placer within the structure's reach of
//! that middle, kept off the world's top and bottom by the padding.

use super::super::BoundingBox;
use super::super::template::Rotation;
use super::{Assembler, Piece, Variant, data, middle};
use crate::random::JavaRandom;
use crate::terrain::MIN_Y;

const MAX_Y: i32 = 319;

pub enum StartHeight {
    Constant(i32),
    /// Both ends included (`UniformHeight`: `nextInt(max - min + 1) + min`).
    Uniform(i32, i32),
}

pub struct V2Start {
    pub pool: &'static str,
    pub max_depth: i32,
    pub horizontal: i32,
    pub vertical: i32,
    pub height: StartHeight,
    /// `heightmap_projection`: the start height counts from the surface.
    pub project: bool,
    /// `dimension_padding`, top and bottom.
    pub padding: i32,
    /// The structure's pool alias set in the jigsaw data.
    pub aliases: Option<&'static str>,
}

impl V2Start {
    /// The start piece at its height and the structure's position (the piece's middle at the
    /// start height), which its biome test reads.
    pub fn start_piece(&self, random: &mut JavaRandom, chunk: (i32, i32), height: &dyn Fn(i32, i32) -> i32) -> (Piece, (i32, i32, i32)) {
        let start_y = match self.height {
            StartHeight::Constant(y) => y,
            StartHeight::Uniform(lo, hi) => random.next_int(hi - lo + 1) + lo,
        };
        let rotation = Rotation::from_index(random.next_int(4));
        let pool = data().pool(self.pool).expect("v2 start pool");
        let element = pool.list[random.next_int(pool.list.len() as i32) as usize];
        let mut piece = Piece::new(element, (chunk.0 * 16, start_y, chunk.1 * 16), rotation, 0);
        let b = piece.bb;
        let (x, z) = (middle(b.x0, b.x1), middle(b.z0, b.z1));
        let y = if self.project { start_y + height(x, z) } else { start_y };
        piece.shift_y(y - 1 - b.y0);
        (piece, (x, y, z))
    }

    pub fn assemble(&self, random: &mut JavaRandom, level_seed: i64, chunk: (i32, i32), height: &dyn Fn(i32, i32) -> i32) -> Vec<Piece> {
        let (piece, (x, y, z)) = self.start_piece(random, chunk, height);
        let start_y = if self.project { y - height(x, z) } else { y };
        let aliases = self.resolve_aliases(level_seed, (chunk.0 * 16, start_y, chunk.1 * 16));
        self.grow(piece, (x, y, z), aliases, random, height)
    }

    /// Java's `/place jigsaw` start (`JigsawPlacement.addPieces` given a start jigsaw name): a
    /// rotation and an element of the pool, placed so its first shuffled jigsaw named `target` is
    /// at `at`, then sunk one as every v2 start; None when the element has no such jigsaw.
    pub fn named_start(&self, target: &str, at: (i32, i32, i32), random: &mut JavaRandom) -> Option<(Piece, (i32, i32, i32))> {
        let pool = data().pool(self.pool).filter(|p| !p.list.is_empty())?;
        let rotation = Rotation::from_index(random.next_int(4));
        let element = pool.list[random.next_int(pool.list.len() as i32) as usize];
        let mut jigsaws = element.jigsaws((0, 0, 0), rotation);
        random.shuffle(&mut jigsaws);
        let jigsaw = jigsaws.iter().find(|j| j.name == target)?;
        let mut piece = Piece::new(element, (at.0 - jigsaw.pos.0, at.1 - jigsaw.pos.1, at.2 - jigsaw.pos.2), rotation, 0);
        piece.shift_y(-1);
        let b = piece.bb;
        Some((piece, (middle(b.x0, b.x1), at.1, middle(b.z0, b.z1))))
    }

    /// The queued placer from `piece`, within the structure's reach of `centre`, kept off the
    /// world's top and bottom by the padding.
    pub fn grow(&self, piece: Piece, (x, y, z): (i32, i32, i32), aliases: Vec<(&'static str, &'static str)>, random: &mut JavaRandom, height: &dyn Fn(i32, i32) -> i32) -> Vec<Piece> {
        let (h, v, pad) = (self.horizontal, self.vertical, self.padding);
        let limit = BoundingBox::new(x - h, (y - v).max(MIN_Y + pad), z - h, x + h, (y + v).min(MAX_Y - pad), z + h);
        let mut assembler = Assembler::new(Variant::V2, self.max_depth, random, height);
        assembler.aliases = aliases;
        assembler.run_queue(piece, limit);
        assembler.pieces
    }

    /// Java's `PoolAliasLookup.create` (in 144ce7f40): one choice per binding, drawn from the
    /// level seed forked at the start chunk's corner and start height.
    fn resolve_aliases(&self, level_seed: i64, (x, y, z): (i32, i32, i32)) -> Vec<(&'static str, &'static str)> {
        let Some(bindings) = self.aliases.and_then(|set| data().bindings.get(set)) else { return Vec::new() };
        let mut random = JavaRandom::new(level_seed).fork_at(x, y, z);
        let mut resolved = Vec::new();
        for choices in bindings {
            let pick = if choices.len() > 1 { random.next_int(choices.len() as i32) as usize } else { 0 };
            resolved.extend(choices[pick].iter().map(|(from, to)| (from.as_str(), to.as_str())));
        }
        resolved
    }
}
