//! The start of a v2 jigsaw structure (`br::worldgen::JigsawAssembler::construct` 144ce64d0, from
//! the structure's generate 144ce7f40): with the start chunk's large-feature `java.util.Random`, a
//! rotation and a start element, its bottom one below the start height (plus the surface at its
//! middle when projected), then the queued placer within the structure's reach of that middle.

use super::super::BoundingBox;
use super::super::template::Rotation;
use super::{Assembler, Piece, Variant, data, middle};
use crate::random::JavaRandom;
use crate::terrain::MIN_Y;

const MAX_Y: i32 = 319;

pub struct V2Start {
    pub pool: &'static str,
    pub max_depth: i32,
    pub horizontal: i32,
    pub vertical: i32,
    /// `start_height`, already sampled.
    pub start_y: i32,
    /// `heightmap_projection`: the start height counts from the surface.
    pub project: bool,
}

impl V2Start {
    /// The start piece at its height and the structure's position (the piece's middle at the
    /// start height), which its biome test reads.
    pub fn start_piece(&self, random: &mut JavaRandom, chunk: (i32, i32), height: &dyn Fn(i32, i32) -> i32) -> (Piece, (i32, i32, i32)) {
        let rotation = Rotation::from_index(random.next_int(4));
        let pool = data().pool(self.pool).expect("v2 start pool");
        let element = pool.list[random.next_int(pool.list.len() as i32) as usize];
        let mut piece = Piece::new(element, (chunk.0 * 16, self.start_y, chunk.1 * 16), rotation, 0);
        let b = piece.bb;
        let (x, z) = (middle(b.x0, b.x1), middle(b.z0, b.z1));
        let y = if self.project { self.start_y + height(x, z) } else { self.start_y };
        piece.shift_y(y - 1 - b.y0);
        (piece, (x, y, z))
    }

    pub fn assemble(&self, random: &mut JavaRandom, chunk: (i32, i32), height: &dyn Fn(i32, i32) -> i32) -> Vec<Piece> {
        let (piece, (x, y, z)) = self.start_piece(random, chunk, height);
        let (h, v) = (self.horizontal, self.vertical);
        let limit = BoundingBox::new(x - h, (y - v).max(MIN_Y), z - h, x + h, (y + v).min(MAX_Y), z + h);
        let mut assembler = Assembler::new(Variant::V2, self.max_depth, random, height);
        assembler.run_queue(piece, limit);
        assembler.pieces
    }
}
