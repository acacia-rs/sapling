//! Nether fortresses (`NetherFortressFeature`, ctor 148321b80; start 148321f50): Java 1.16's
//! `NetherFortressPieces` on the start's MT, with BDS's own turns (layout.rs). Post-processes
//! write plain blocks over anything; `rejoin_connections` joins the fences afterwards.

mod bridge;
mod castle;
mod corridor;
mod layout;

use super::BoundingBox;
use super::canvas::Canvas;
use super::piece::Frame;
use crate::block::Block;
use crate::random::Mt19937;
pub use layout::generate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    BridgeStraight,
    BridgeCrossing,
    BridgeEndFiller,
    RoomCrossing,
    StairsRoom,
    MonsterThrone,
    CastleEntrance,
    SmallCorridor,
    SmallCorridorCrossing,
    RightTurn,
    LeftTurn,
    CorridorStairs,
    TBalcony,
    StalkRoom,
}

#[derive(Debug, Clone)]
pub struct Piece {
    pub kind: Kind,
    pub frame: Frame,
    /// Turns: a chest is wanted (never cleared: the cell's chest stops a second one).
    pub chest: bool,
    /// The filler's own MT seed.
    pub seed: u32,
}

#[derive(Debug, Clone)]
pub struct Fortress {
    pub pieces: Vec<Piece>,
    pub bb: BoundingBox,
}

impl Fortress {
    /// The pieces reaching the chunk, in start order; `random` is the chunk's structure generator
    /// (the chests' loot draws).
    pub fn post_process(&self, canvas: &mut Canvas, random: &mut Mt19937) {
        let column = canvas.bb;
        for (index, p) in self.pieces.iter().enumerate().filter(|(_, p)| p.frame.bb.intersects(&column)) {
            super::trace::label(|| format!("fortress piece {index} {:?} {:?}", p.kind, p.frame.bb));
            let f = &p.frame;
            match p.kind {
                Kind::BridgeStraight => bridge::straight(f, canvas),
                Kind::BridgeCrossing => bridge::crossing(f, canvas),
                Kind::BridgeEndFiller => bridge::end_filler(f, canvas, p.seed),
                Kind::RoomCrossing => bridge::room_crossing(f, canvas),
                Kind::StairsRoom => bridge::stairs_room(f, canvas),
                Kind::MonsterThrone => bridge::monster_throne(f, canvas),
                Kind::CastleEntrance => castle::entrance(f, canvas),
                Kind::StalkRoom => castle::stalk_room(f, canvas),
                Kind::SmallCorridor => corridor::small(f, canvas),
                Kind::SmallCorridorCrossing => corridor::crossing(f, canvas),
                Kind::RightTurn => corridor::turn(f, canvas, random, p.chest, false),
                Kind::LeftTurn => corridor::turn(f, canvas, random, p.chest, true),
                Kind::CorridorStairs => corridor::stairs(f, canvas),
                Kind::TBalcony => corridor::balcony(f, canvas),
            }
        }
    }
}

type Pos = (i32, i32, i32);

fn fill(f: &Frame, canvas: &mut Canvas, lo: Pos, hi: Pos, block: Block) {
    f.generate_box(canvas, lo, hi, block, block, false);
}

fn bricks(f: &Frame, canvas: &mut Canvas, lo: Pos, hi: Pos) {
    fill(f, canvas, lo, hi, Block::NetherBrick);
}

fn fences(f: &Frame, canvas: &mut Canvas, lo: Pos, hi: Pos) {
    fill(f, canvas, lo, hi, Block::NetherBrickFence0);
}

fn air(f: &Frame, canvas: &mut Canvas, lo: Pos, hi: Pos) {
    fill(f, canvas, lo, hi, Block::Air);
}

/// `fillColumnDown` with nether bricks from just below the piece.
fn down(f: &Frame, canvas: &mut Canvas, x: i32, z: i32) {
    f.fill_column_down(canvas, Block::NetherBrick, x, -1, z);
}

/// Nether brick stairs facing legacy `data` (0 east, 1 west, 2 south, 3 north) in the piece's orientation.
fn stairs(f: &Frame, data: u8) -> Block {
    let family = [Block::NetherBrickStairsEast, Block::NetherBrickStairsWest, Block::NetherBrickStairsSouth, Block::NetherBrickStairsNorth];
    super::scattered::stairs(f, family, data)
}
