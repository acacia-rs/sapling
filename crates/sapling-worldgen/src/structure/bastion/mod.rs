//! Bastion remnants (`createStructureStart` 148311de0 -> `BastionPiece::addPieces` 141edf3d0): a
//! rotation, one of four start pools and its element from the start's generator, then the ancient
//! city's queued jigsaw placer (depth 15, within 80 of the start piece's centre) with an empty anchor,
//! the start piece at y 32 (the piece class's height, 141edf980, is sea level + 1). Pieces are the
//! city's class but plugged with netherrack (141edf990), down to the Nether's floor.

#[cfg(test)]
mod tests;

use super::BoundingBox;
use super::canvas::Canvas;
use super::jigsaw::{Assembler, Piece, PieceClass, Variant, data};
use super::template::{Rotation, named};
use crate::block::Block;
use crate::random::Mt19937;

const START_POOLS: [&str; 4] = ["bastion/bridge/start", "bastion/treasure/starters", "bastion/hoglin_stable/origin", "bastion/units/air_base"];
const HEIGHT: i32 = crate::nether::SEA_LEVEL + 1;
const MAX_DEPTH: i32 = 15;
const MAX_DISTANCE: i32 = 80;
/// Pieces stay within 80 blocks of the start piece's centre plus a piece.
pub const REACH: i32 = 7;

pub struct Bastion {
    pub pieces: Vec<Piece>,
}

impl Bastion {
    /// The bastion of start chunk `start`; `random` is the region's generator as the shared Nether
    /// selector (1480b9cf0) leaves it, after its `nextInt(6)`.
    pub fn generate(random: &mut Mt19937, start: (i32, i32)) -> Bastion {
        let rotation = Rotation::from_index(random.next_int(4));
        let pool = data().pool(START_POOLS[random.next_int(4) as usize]).expect("bastion start pools");
        let element = pool.list[random.next_int(pool.list.len() as i32) as usize];
        let height = |_: i32, _: i32| HEIGHT;
        let mut assembler = Assembler::new(Variant::Queued, MAX_DEPTH, random, &height);
        assembler.start_queued(element, (start.0 * 16, 0, start.1 * 16), rotation, "", MAX_DISTANCE);
        Bastion { pieces: assembler.pieces }
    }

    /// Whether a piece meets `bb`.
    pub fn reaches(&self, bb: &BoundingBox) -> bool {
        self.pieces.iter().any(|p| p.bb.intersects(bb))
    }

    /// The pieces meeting the canvas's chunk, in assembly order; `seed` is the chunk's structure
    /// seed and `random` the chunk's structure generator seeded with it, which chests draw from.
    pub fn post_process(&self, canvas: &mut Canvas, seed: u32, random: &mut Mt19937) {
        let column = canvas.bb;
        for piece in self.pieces.iter().filter(|p| p.bb.intersects(&column)) {
            piece.post_process(canvas, seed, random, &BastionClass, &|_, _| 0);
        }
    }
}

/// Every bastion's pieces meeting the canvas's chunk, one generator for the chunk as the ancient
/// city's feature draws.
pub fn place(canvas: &mut Canvas, bastions: &[&Bastion], seed: u32) {
    let mut random = Mt19937::new(seed);
    for bastion in bastions {
        bastion.post_process(canvas, seed, &mut random);
    }
}

/// Bastion pieces (`BastionPiece`, vtable 14a783ca0): no fill, netherrack plugs, the Nether's floor.
pub struct BastionClass;

impl PieceClass for BastionClass {
    fn plug(&self, _block: Block) -> Block {
        named("netherrack", "", false)
    }

    fn floor(&self) -> i32 {
        0
    }
}
