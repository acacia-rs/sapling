//! Fortress layout (`createStructureStart` 148321f50, `generateAndAddPiece` 148323e70): Java 1.16's
//! weighted pieces, every draw on the start's MT (no generator copy, unlike strongholds), and:
//! - a child past 112 blocks of the start becomes a filler kept as a piece and pending (Java drops it);
//! - a pick whose piece does not fit falls through to the next weights with the roll still negative
//!   (Java's quirk, kept);
//! - orientations are BDS's (0 south, 1 west, 2 north, 3 east).

use super::{Fortress, Kind, Piece};
use crate::random::Mt19937;
use crate::structure::BoundingBox;
use crate::structure::piece::{EAST, Frame, NORTH, SOUTH, WEST, orient_box};

/// `Dimension` +0xcc, read as the Nether's sea level: the layout moves into sea+16..sea+38.
const SEA_LEVEL: i32 = 32;

/// (kind, weight, most placed or 0 for any, may follow itself), in BDS's list order.
const BRIDGE: [(Kind, i32, i32, bool); 6] = [
    (Kind::BridgeStraight, 30, 0, true),
    (Kind::BridgeCrossing, 10, 4, false),
    (Kind::RoomCrossing, 10, 4, false),
    (Kind::StairsRoom, 10, 3, false),
    (Kind::MonsterThrone, 5, 2, false),
    (Kind::CastleEntrance, 5, 1, false),
];
const CASTLE: [(Kind, i32, i32, bool); 7] = [
    (Kind::SmallCorridor, 25, 0, true),
    (Kind::SmallCorridorCrossing, 15, 5, false),
    (Kind::RightTurn, 5, 10, false),
    (Kind::LeftTurn, 5, 10, false),
    (Kind::CorridorStairs, 10, 3, true),
    (Kind::TBalcony, 7, 2, false),
    (Kind::StalkRoom, 5, 2, false),
];

struct Weight {
    kind: Kind,
    weight: i32,
    placed: i32,
    max: i32,
    in_row: bool,
}

fn weights(list: &[(Kind, i32, i32, bool)]) -> Vec<Weight> {
    list.iter().map(|&(kind, weight, max, in_row)| Weight { kind, weight, placed: 0, max, in_row }).collect()
}

fn frame(bb: BoundingBox, orientation: u8, depth: i32) -> Frame {
    Frame { bb, orientation: Some(orientation), depth, keep: |_| false }
}

/// The start's pieces; `random` is the selector's MT after its grid and pick draws.
pub fn generate(random: &mut Mt19937, (cx, cz): (i32, i32)) -> Fortress {
    let orientation = random.next_int(4) as u8;
    let (x, z) = (cx * 16 + 2, cz * 16 + 2);
    let bb = BoundingBox::new(x, 64, z, x + 18, 73, z + 18);
    let start = Piece { kind: Kind::BridgeCrossing, frame: frame(bb, orientation, 0), chest: false, seed: 0 };
    let mut b = Builder { pieces: vec![start], pending: Vec::new(), bridge: weights(&BRIDGE), castle: weights(&CASTLE), previous: None, start: bb };
    b.add_children(0, random);
    while !b.pending.is_empty() {
        let index = b.pending.remove(random.next_int(b.pending.len() as i32) as usize);
        b.add_children(index, random);
    }
    let bb = b.pieces.iter().skip(1).fold(b.pieces[0].frame.bb, |bb, p| bb.union(&p.frame.bb));
    let low = SEA_LEVEL + 16;
    let span = 22 - (bb.y1 - bb.y0);
    let y0 = if span > 1 { low + random.next_int(span) } else { low };
    let shift = y0 - bb.y0;
    let mut pieces = b.pieces;
    pieces.iter_mut().for_each(|p| p.frame.bb.move_y(shift));
    let mut bb = bb;
    bb.move_y(shift);
    Fortress { pieces, bb }
}

struct Builder {
    pieces: Vec<Piece>,
    pending: Vec<usize>,
    bridge: Vec<Weight>,
    castle: Vec<Weight>,
    previous: Option<Kind>,
    start: BoundingBox,
}

impl Builder {
    fn add_children(&mut self, index: usize, random: &mut Mt19937) {
        let p = self.pieces[index].frame.clone();
        match self.pieces[index].kind {
            Kind::BridgeCrossing => {
                self.forward(random, &p, 8, 3, false);
                self.left(random, &p, 3, 8, false);
                self.right(random, &p, 3, 8, false);
            }
            Kind::BridgeStraight => self.forward(random, &p, 1, 3, false),
            Kind::RoomCrossing => {
                self.forward(random, &p, 2, 0, false);
                self.left(random, &p, 0, 2, false);
                self.right(random, &p, 0, 2, false);
            }
            Kind::StairsRoom => self.right(random, &p, 6, 2, false),
            Kind::CastleEntrance => self.forward(random, &p, 5, 3, true),
            Kind::StalkRoom => {
                self.forward(random, &p, 5, 3, true);
                self.forward(random, &p, 5, 11, true);
            }
            Kind::SmallCorridor | Kind::CorridorStairs => self.forward(random, &p, 1, 0, true),
            Kind::SmallCorridorCrossing => {
                self.forward(random, &p, 1, 0, true);
                self.left(random, &p, 0, 1, true);
                self.right(random, &p, 0, 1, true);
            }
            Kind::RightTurn => self.right(random, &p, 0, 1, true),
            Kind::LeftTurn => self.left(random, &p, 0, 1, true),
            Kind::TBalcony => {
                let offset = if matches!(p.orientation, Some(WEST | NORTH)) { 5 } else { 1 };
                let castle = random.next_int(8) > 0;
                self.left(random, &p, 0, offset, castle);
                let castle = random.next_int(8) > 0;
                self.right(random, &p, 0, offset, castle);
            }
            Kind::BridgeEndFiller | Kind::MonsterThrone => {}
        }
    }

    fn forward(&mut self, random: &mut Mt19937, f: &Frame, dx: i32, dy: i32, castle: bool) {
        let (b, o) = (f.bb, f.orientation.unwrap_or(SOUTH));
        let (x, z) = match o {
            SOUTH => (b.x0 + dx, b.z1 + 1),
            WEST => (b.x0 - 1, b.z0 + dx),
            NORTH => (b.x0 + dx, b.z0 - 1),
            _ => (b.x1 + 1, b.z0 + dx),
        };
        self.add(random, (x, b.y0 + dy, z), o, f.depth, castle);
    }

    fn left(&mut self, random: &mut Mt19937, f: &Frame, dy: i32, dz: i32, castle: bool) {
        let b = f.bb;
        match f.orientation.unwrap_or(SOUTH) {
            SOUTH | NORTH => self.add(random, (b.x0 - 1, b.y0 + dy, b.z0 + dz), WEST, f.depth, castle),
            _ => self.add(random, (b.x0 + dz, b.y0 + dy, b.z0 - 1), NORTH, f.depth, castle),
        }
    }

    fn right(&mut self, random: &mut Mt19937, f: &Frame, dy: i32, dz: i32, castle: bool) {
        let b = f.bb;
        match f.orientation.unwrap_or(SOUTH) {
            SOUTH | NORTH => self.add(random, (b.x1 + 1, b.y0 + dy, b.z0 + dz), EAST, f.depth, castle),
            _ => self.add(random, (b.x0 + dz, b.y0 + dy, b.z1 + 1), SOUTH, f.depth, castle),
        }
    }

    /// `generateAndAddPiece`: whatever it makes goes on the piece list and the pending list.
    fn add(&mut self, random: &mut Mt19937, pos: (i32, i32, i32), dir: u8, depth: i32, castle: bool) {
        let far = (pos.0 - self.start.x0).abs() > 112 || (pos.2 - self.start.z0).abs() > 112;
        let piece = if far { self.create(Kind::BridgeEndFiller, random, pos, dir, depth) } else { self.choose(random, pos, dir, depth + 1, castle) };
        if let Some(piece) = piece {
            self.pending.push(self.pieces.len());
            self.pieces.push(piece);
        }
    }

    fn choose(&mut self, random: &mut Mt19937, pos: (i32, i32, i32), dir: u8, depth: i32, castle: bool) -> Option<Piece> {
        let list = if castle { &self.castle } else { &self.bridge };
        let total: i32 = list.iter().map(|w| w.weight).sum();
        let open = list.iter().any(|w| w.max > 0 && w.placed < w.max);
        if open && total > 0 && depth <= 30 {
            for _ in 0..5 {
                let mut roll = random.next_int(total);
                let len = if castle { self.castle.len() } else { self.bridge.len() };
                for index in 0..len {
                    let w = &(if castle { &self.castle } else { &self.bridge })[index];
                    roll -= w.weight;
                    if roll >= 0 {
                        continue;
                    }
                    if w.max != 0 && w.placed >= w.max || self.previous == Some(w.kind) && !w.in_row {
                        break;
                    }
                    let kind = w.kind;
                    if let Some(piece) = self.create(kind, random, pos, dir, depth) {
                        let list = if castle { &mut self.castle } else { &mut self.bridge };
                        list[index].placed += 1;
                        self.previous = Some(kind);
                        if list[index].max != 0 && list[index].placed >= list[index].max {
                            list.remove(index);
                        }
                        return Some(piece);
                    }
                }
            }
        }
        self.create(Kind::BridgeEndFiller, random, pos, dir, depth)
    }

    /// `createPiece`: the box above y 10 and clear of every piece, then the piece's draws.
    fn create(&self, kind: Kind, random: &mut Mt19937, pos: (i32, i32, i32), dir: u8, depth: i32) -> Option<Piece> {
        let (offset, size) = match kind {
            Kind::BridgeStraight => ((-1, -3, 0), (5, 10, 19)),
            Kind::BridgeCrossing => ((-8, -3, 0), (19, 10, 19)),
            Kind::BridgeEndFiller => ((-1, -3, 0), (5, 10, 8)),
            Kind::RoomCrossing => ((-2, 0, 0), (7, 9, 7)),
            Kind::StairsRoom => ((-2, 0, 0), (7, 11, 7)),
            Kind::MonsterThrone => ((-2, 0, 0), (7, 8, 9)),
            Kind::CastleEntrance | Kind::StalkRoom => ((-5, -3, 0), (13, 14, 13)),
            Kind::SmallCorridor | Kind::SmallCorridorCrossing | Kind::RightTurn | Kind::LeftTurn => ((-1, 0, 0), (5, 7, 5)),
            Kind::CorridorStairs => ((-1, -7, 0), (5, 14, 10)),
            Kind::TBalcony => ((-3, 0, 0), (9, 7, 9)),
        };
        let bb = orient_box(pos, offset, size, dir);
        if bb.y0 <= 10 || self.pieces.iter().any(|p| p.frame.bb.intersects(&bb)) {
            return None;
        }
        let mut piece = Piece { kind, frame: frame(bb, dir, depth), chest: false, seed: 0 };
        match kind {
            Kind::RightTurn | Kind::LeftTurn => piece.chest = random.next_int(3) == 0,
            Kind::BridgeEndFiller => piece.seed = random.next_int_unbounded() as u32,
            _ => {}
        }
        Some(piece)
    }
}
