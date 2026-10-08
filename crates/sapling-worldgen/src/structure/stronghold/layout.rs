//! Stronghold layout (`createStructureStart` 148375450): Java's legacy weighted pieces on the start
//! generator, with BDS's own turns:
//! - `generatePieceFromSmallDoor` (14809b270) works on a copy of the generator, so the parent never
//!   advances and siblings draw the same numbers; only the pending pick moves it.
//! - The weighted pick is `genrand % total`; Java's retry until a portal room exists is replaced by
//!   moving the last piece's slot to a portal room (14809c9e0).

use super::common::{NORTH_OR_EAST, orient_box, random_door};
use super::{Door, Kind, Piece, Stronghold};
use crate::random::Mt19937;
use crate::structure::BoundingBox;
use crate::structure::piece::{EAST, Frame, NORTH, SOUTH, WEST};

const SEA_LEVEL: i32 = 63;
const MIN_Y: i32 = -64;

/// (kind, weight, most placed or 0 for any, least depth), in BDS's list order.
const WEIGHTS: [(Kind, i32, i32, i32); 11] = [
    (Kind::Straight, 40, 0, 0),
    (Kind::PrisonHall, 5, 5, 0),
    (Kind::LeftTurn, 20, 0, 0),
    (Kind::RightTurn, 20, 0, 0),
    (Kind::RoomCrossing, 10, 6, 0),
    (Kind::StraightStairsDown, 5, 5, 0),
    (Kind::StairsDown, 5, 5, 0),
    (Kind::FiveCrossing, 5, 4, 0),
    (Kind::ChestCorridor, 5, 4, 0),
    (Kind::Library, 10, 2, 5),
    (Kind::PortalRoom, 10, 1, 6),
];

struct Weight {
    kind: Kind,
    weight: i32,
    placed: i32,
    max: i32,
    min_depth: i32,
}

impl Weight {
    fn exhausted(&self) -> bool {
        self.max != 0 && self.placed >= self.max
    }
}

fn frame(bb: BoundingBox, orientation: u8, depth: i32) -> Frame {
    Frame { bb, orientation: Some(orientation), depth, keep: |_| false }
}

/// The start's pieces; `random` is the start chunk's blueprint generator.
pub fn generate(random: &mut Mt19937, (cx, cz): (i32, i32)) -> Stronghold {
    let orientation = random.next_int(4) as u8;
    let (x, z) = (cx * 16, cz * 16);
    let bb = BoundingBox::new(x + 2, 64, z + 2, x + 6, 74, z + 6);
    let start = Piece { kind: Kind::StairsDown, frame: frame(bb, orientation, 0), door: Door::Opening, flags: [true, false, false, false], value: 0 };
    let weights = WEIGHTS.iter().map(|&(kind, weight, max, min_depth)| Weight { kind, weight, placed: 0, max, min_depth }).collect();
    let mut b = Builder { pieces: vec![start], pending: Vec::new(), weights, previous: None, imposed: None, start: bb };
    b.add_children(0, random);
    while !b.pending.is_empty() {
        let index = b.pending.remove(random.next_int(b.pending.len() as i32) as usize);
        b.add_children(index, random);
    }
    if !b.pieces.iter().any(|p| p.kind == Kind::PortalRoom) {
        b.portal_fallback();
    }
    let mut bb = b.pieces[0].frame.bb;
    b.pieces.iter().for_each(|p| bb = bb.union(&p.frame.bb));
    let mut top = bb.y1 - bb.y0 + MIN_Y + 2;
    if top < SEA_LEVEL - 10 {
        top += random.next_int(SEA_LEVEL - 10 - top);
    }
    let shift = top - bb.y1;
    b.pieces.iter_mut().for_each(|p| p.frame.bb.move_y(shift));
    bb.move_y(shift);
    Stronghold { pieces: b.pieces, bb }
}

struct Builder {
    pieces: Vec<Piece>,
    pending: Vec<usize>,
    weights: Vec<Weight>,
    previous: Option<Kind>,
    imposed: Option<Kind>,
    start: BoundingBox,
}

impl Builder {
    fn add_children(&mut self, index: usize, random: &mut Mt19937) {
        let p = self.pieces[index].clone();
        let o = p.frame.orientation.unwrap_or(SOUTH);
        let north_or_east = NORTH_OR_EAST.contains(&o);
        match p.kind {
            Kind::StairsDown => {
                if p.flags[0] {
                    self.imposed = Some(Kind::FiveCrossing);
                }
                self.forward(random, &p, 1);
            }
            Kind::Straight => {
                self.forward(random, &p, 1);
                if p.flags[0] {
                    self.left(random, &p, 1, 2);
                }
                if p.flags[1] {
                    self.right(random, &p, 1, 2);
                }
            }
            Kind::ChestCorridor | Kind::PrisonHall | Kind::StraightStairsDown => self.forward(random, &p, 1),
            Kind::LeftTurn if north_or_east => self.left(random, &p, 1, 1),
            Kind::LeftTurn => self.right(random, &p, 1, 1),
            Kind::RightTurn if north_or_east => self.right(random, &p, 1, 1),
            Kind::RightTurn => self.left(random, &p, 1, 1),
            Kind::RoomCrossing => {
                self.forward(random, &p, 4);
                self.left(random, &p, 1, 4);
                self.right(random, &p, 1, 4);
            }
            Kind::FiveCrossing => {
                let (low, high) = if o == WEST || o == NORTH { (5, 3) } else { (3, 5) };
                self.forward(random, &p, 5);
                let [left_low, left_high, right_low, right_high] = p.flags;
                if left_low {
                    self.left(random, &p, low, 1);
                }
                if left_high {
                    self.left(random, &p, high, 7);
                }
                if right_low {
                    self.right(random, &p, low, 1);
                }
                if right_high {
                    self.right(random, &p, high, 7);
                }
            }
            _ => {}
        }
    }

    fn forward(&mut self, random: &mut Mt19937, p: &Piece, offset: i32) {
        let (b, o) = (p.frame.bb, p.frame.orientation.unwrap_or(SOUTH));
        let (x, z) = match o {
            SOUTH => (b.x0 + offset, b.z1 + 1),
            WEST => (b.x0 - 1, b.z0 + offset),
            NORTH => (b.x0 + offset, b.z0 - 1),
            _ => (b.x1 + 1, b.z0 + offset),
        };
        self.piece_at_small_door(random, (x, b.y0 + 1, z), o, p.frame.depth);
    }

    fn left(&mut self, random: &mut Mt19937, p: &Piece, dy: i32, offset: i32) {
        let b = p.frame.bb;
        match p.frame.orientation.unwrap_or(SOUTH) {
            SOUTH | NORTH => self.piece_at_small_door(random, (b.x0 - 1, b.y0 + dy, b.z0 + offset), WEST, p.frame.depth),
            _ => self.piece_at_small_door(random, (b.x0 + offset, b.y0 + dy, b.z0 - 1), NORTH, p.frame.depth),
        }
    }

    fn right(&mut self, random: &mut Mt19937, p: &Piece, dy: i32, offset: i32) {
        let b = p.frame.bb;
        match p.frame.orientation.unwrap_or(SOUTH) {
            SOUTH | NORTH => self.piece_at_small_door(random, (b.x1 + 1, b.y0 + dy, b.z0 + offset), EAST, p.frame.depth),
            _ => self.piece_at_small_door(random, (b.x0 + offset, b.y0 + dy, b.z1 + 1), SOUTH, p.frame.depth),
        }
    }

    fn piece_at_small_door(&mut self, random: &Mt19937, pos: (i32, i32, i32), dir: u8, depth: i32) {
        if depth > 50 || (pos.0 - self.start.x0).abs() > 112 || (pos.2 - self.start.z0).abs() > 112 {
            return;
        }
        let mut copy = random.clone();
        if let Some(piece) = self.choose(&mut copy, pos, dir, depth + 1) {
            self.pending.push(self.pieces.len());
            self.pieces.push(piece);
        }
    }

    fn choose(&mut self, random: &mut Mt19937, pos: (i32, i32, i32), dir: u8, depth: i32) -> Option<Piece> {
        if let Some(kind) = self.imposed.take()
            && let Some(piece) = self.create(kind, random, pos, dir, depth)
        {
            return Some(piece);
        }
        let total: i32 = self.weights.iter().map(|w| w.weight).sum();
        if !self.weights.iter().any(|w| w.max > 0 && w.placed < w.max) || total <= 0 {
            return None;
        }
        for _ in 0..5 {
            let mut roll = (random.next_u32() % total as u32) as i32;
            for index in 0..self.weights.len() {
                let w = &self.weights[index];
                roll -= w.weight;
                if roll >= 0 {
                    continue;
                }
                if w.exhausted() || depth < w.min_depth || self.previous == Some(w.kind) && self.weights.len() > 1 {
                    break;
                }
                let kind = w.kind;
                if let Some(piece) = self.create(kind, random, pos, dir, depth) {
                    let w = &mut self.weights[index];
                    w.placed += 1;
                    self.previous = Some(kind);
                    if w.exhausted() {
                        self.weights.remove(index);
                    }
                    return Some(piece);
                }
            }
        }
        self.filler(pos, dir, depth)
    }

    fn collides(&self, bb: &BoundingBox) -> bool {
        self.pieces.iter().any(|p| p.frame.bb.intersects(bb))
    }

    /// `createPiece` (148099bd0): the box above y 10 and clear of every piece, then the draws.
    fn create(&self, kind: Kind, random: &mut Mt19937, pos: (i32, i32, i32), dir: u8, depth: i32) -> Option<Piece> {
        let sized = |offset, size| Some(orient_box(pos, offset, size, dir)).filter(|bb| bb.y0 > 10 && !self.collides(bb));
        let bb = match kind {
            Kind::Straight | Kind::ChestCorridor => sized((-1, -1, 0), (5, 5, 7)),
            Kind::PrisonHall => sized((-1, -1, 0), (9, 5, 11)),
            Kind::LeftTurn | Kind::RightTurn => sized((-1, -1, 0), (5, 5, 5)),
            Kind::RoomCrossing => sized((-4, -1, 0), (11, 7, 11)),
            Kind::StraightStairsDown => sized((-1, -7, 0), (5, 11, 8)),
            Kind::StairsDown => sized((-1, -7, 0), (5, 11, 5)),
            Kind::FiveCrossing => sized((-4, -3, 0), (10, 9, 11)),
            Kind::Library => sized((-4, -1, 0), (14, 11, 15)).or_else(|| sized((-4, -1, 0), (14, 6, 15))),
            Kind::PortalRoom => sized((-4, -1, 0), (11, 8, 16)),
            Kind::Filler => None,
        }?;
        let door = if kind == Kind::PortalRoom { Door::Opening } else { random_door(random) };
        let mut piece = Piece { kind, frame: frame(bb, dir, depth), door, flags: [false; 4], value: 0 };
        match kind {
            Kind::Straight => piece.flags = [random.next_int(2) == 0, random.next_int(2) == 0, false, false],
            Kind::RoomCrossing => piece.value = random.next_int(5),
            Kind::FiveCrossing => {
                let left_low = random.next_boolean();
                let left_high = random.next_boolean();
                let right_low = random.next_boolean();
                piece.flags = [left_low, left_high, right_low, random.next_int(3) > 0];
            }
            Kind::Library => piece.flags[0] = bb.y1 - bb.y0 > 5,
            _ => {}
        }
        Some(piece)
    }

    /// `FillerCorridor::findPieceBox`: up to the first piece in the way, when it starts level.
    fn filler(&self, pos: (i32, i32, i32), dir: u8, depth: i32) -> Option<Piece> {
        let bb = orient_box(pos, (-1, -1, 0), (5, 5, 4), dir);
        let other = self.pieces.iter().find(|p| p.frame.bb.intersects(&bb))?.frame.bb;
        if other.y0 != bb.y0 {
            return None;
        }
        let length = (1..=3).rev().find(|&j| !other.intersects(&orient_box(pos, (-1, -1, 0), (5, 5, j - 1), dir)))?;
        let bb = orient_box(pos, (-1, -1, 0), (5, 5, length), dir);
        (bb.y0 > 1).then(|| Piece { kind: Kind::Filler, frame: frame(bb, dir, depth), door: Door::Opening, flags: [false; 4], value: length })
    }

    /// No portal room: the last piece gives way to one beside the piece before it (14809c9e0),
    /// unless that one lies below y 11.
    fn portal_fallback(&mut self) {
        if self.pieces.len() < 2 {
            return;
        }
        let removed = self.pieces.pop().unwrap();
        let o = removed.frame.orientation.unwrap_or(SOUTH);
        let last = &self.pieces[self.pieces.len() - 1].frame;
        let b = last.bb;
        let (x0, z0, x1, z1) = match o {
            SOUTH => ((b.x0 + b.x1) / 2 - 3, b.z1, (b.x0 + b.x1) / 2 + 7, b.z1 + 15),
            WEST => (b.x0 - 15, (b.z0 + b.z1) / 2 - 5, b.x0, (b.z0 + b.z1) / 2 + 5),
            NORTH => ((b.x0 + b.x1) / 2 - 5, b.z0 - 15, (b.x0 + b.x1) / 2 + 5, b.z0),
            _ => (b.x1, (b.z0 + b.z1) / 2 - 3, b.x1 + 15, (b.z0 + b.z1) / 2 + 7),
        };
        if b.y0 < 11 {
            return;
        }
        let bb = BoundingBox::new(x0, b.y0, z0, x1, b.y0 + 7, z1);
        let depth = last.depth;
        self.pieces.push(Piece { kind: Kind::PortalRoom, frame: frame(bb, o, depth), door: Door::Opening, flags: [false; 4], value: 0 });
    }
}
