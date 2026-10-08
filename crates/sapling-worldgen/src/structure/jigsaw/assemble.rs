//! BDS's two `JigsawPlacement`s. Legacy (villages: ctor 141ee6920, addPieces 141ee6a70, expand
//! 141ee7000, candidates 141ee7770) grows depth first and only keeps boxes from overlapping in x and
//! z. Queued (ancient cities: addPieces 141edde60, expand 141ee4b20, candidates 141ee5600) grows
//! breadth first through a (placement priority, insertion) queue and keeps pieces inside allowed
//! boxes and off occupied ones, a fresh space per jigsaw that attaches inside its piece.

use std::collections::BinaryHeap;

use super::super::BoundingBox;
use super::super::template::Rotation;
use super::element::{Element, JigsawInfo, Projection};
use super::{Data, Piece, Pool};
use crate::random::Mt19937;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Legacy,
    Queued,
}

#[derive(Default)]
struct Space {
    allowed: Vec<BoundingBox>,
    occupied: Vec<BoundingBox>,
}

pub struct Assembler<'a> {
    pub variant: Variant,
    pub max_depth: i32,
    pub data: &'static Data,
    pub random: &'a mut Mt19937,
    /// The piece class's height at a jigsaw (`PoolElementStructurePiece` vt+0x68).
    pub height: &'a dyn Fn(i32, i32) -> i32,
    pub pieces: Vec<Piece>,
    spaces: Vec<Space>,
    /// (placement priority, -insertion, piece, space).
    queue: BinaryHeap<(i32, i64, usize, usize)>,
    pushed: i64,
}

/// BDS's shuffle (141ee53d0, 145e23dc0): for i from 1, swap i with `u32 % (i + 1)` when they differ.
pub fn shuffle<T>(items: &mut [T], random: &mut Mt19937) {
    for i in 1..items.len() {
        let j = (random.next_u32() % (i as u32 + 1)) as usize;
        if j != i {
            items.swap(i, j);
        }
    }
}

impl<'a> Assembler<'a> {
    pub fn new(variant: Variant, max_depth: i32, random: &'a mut Mt19937, height: &'a dyn Fn(i32, i32) -> i32) -> Self {
        let data = super::data();
        Self { variant, max_depth, data, random, height, pieces: Vec::new(), spaces: Vec::new(), queue: BinaryHeap::new(), pushed: 0 }
    }

    /// The queued placer's start (141edde60): the start element turned so its `anchor` jigsaw
    /// lands on `start`'s column, `height(centre) - 1` below the anchor, within `max_distance`.
    pub fn start_queued(&mut self, element: &'static Element, start: (i32, i32, i32), rotation: Rotation, anchor: &str, max_distance: i32) {
        let mut jigsaws = element.jigsaws(start, rotation);
        shuffle(&mut jigsaws, self.random);
        let (dx, dy, dz) = jigsaws.iter().find(|j| j.name == anchor).map_or((0, 0, 0), |j| (start.0 - j.pos.0, j.pos.1 - start.1, start.2 - j.pos.2));
        let origin = (start.0 + dx, 0, start.2 + dz);
        let bb = element.bbox(origin, rotation);
        let h = (self.height)(middle(bb.x0, bb.x1), middle(bb.z0, bb.z1));
        let shift = h - dy - bb.y0 - 1;
        let mut piece = Piece::new(element, origin, rotation, 0);
        piece.shift_y(shift);
        let b = piece.bb;
        let (cx, cy, cz) = (middle(b.x0, b.x1), middle(b.y0, b.y1), middle(b.z0, b.z1));
        let limit = BoundingBox::new(cx - max_distance, cy - max_distance, cz - max_distance, cx + max_distance, cy + max_distance, cz + max_distance);
        self.spaces.push(Space { allowed: vec![limit], occupied: vec![b] });
        self.pieces.push(piece);
        self.enqueue(0, 0, 0);
        while let Some((_, _, index, space)) = self.queue.pop() {
            self.expand(index, space);
        }
    }

    /// The legacy placer's start (141ee6a70): `piece` already at its height, then depth first.
    pub fn start_legacy(&mut self, piece: Piece) {
        self.pieces.push(piece);
        self.expand(0, 0);
    }

    fn enqueue(&mut self, priority: i32, index: usize, space: usize) {
        self.pushed += 1;
        self.queue.push((priority, -self.pushed, index, space));
    }

    /// One piece's jigsaws, shuffled (then stably by selection priority when queued), each tried
    /// against its pool, or only the fallback at the depth limit.
    fn expand(&mut self, index: usize, space: usize) {
        let piece = self.pieces[index].clone();
        if piece.depth > self.max_depth {
            return;
        }
        let mut jigsaws = piece.element.jigsaws(piece.origin, piece.rotation);
        shuffle(&mut jigsaws, self.random);
        if self.variant == Variant::Queued {
            jigsaws.sort_by_key(|j| std::cmp::Reverse(j.selection_priority));
        }
        let mut inside = Vec::new();
        for jigsaw in &jigsaws {
            let Some(pool) = self.data.pool(jigsaw.pool).filter(|p| !p.list.is_empty()) else { continue };
            let fallback = pool.fallback.as_deref().and_then(|f| self.data.pool(f)).filter(|f| !f.list.is_empty());
            if piece.depth == self.max_depth {
                if let Some(fallback) = fallback {
                    self.try_pool(&piece, jigsaw, fallback, space, &mut inside);
                }
            } else if !self.try_pool(&piece, jigsaw, pool, space, &mut inside)
                && let Some(fallback) = fallback
            {
                self.try_pool(&piece, jigsaw, fallback, space, &mut inside);
            }
        }
    }

    /// Tries `pool`'s elements (shuffled), each in shuffled rotations, each of their shuffled
    /// jigsaws that fits `jigsaw`; true once one is placed or an empty element ends the list.
    fn try_pool(&mut self, source: &Piece, jigsaw: &JigsawInfo, pool: &Pool, space: usize, inside: &mut Vec<BoundingBox>) -> bool {
        let src_bb = source.bb;
        let (o, f) = (jigsaw.front.offset(), jigsaw.pos);
        let target = (f.0 + o.0, f.1 + o.1, f.2 + o.2);
        let is_inside = src_bb.contains(target.0, target.1, target.2);
        let surface = (self.height)(jigsaw.pos.0, jigsaw.pos.2) - 1;
        let mut order: Vec<usize> = (0..pool.list.len()).collect();
        shuffle(&mut order, self.random);
        let mut space = space;
        for index in order {
            let element = pool.list[index];
            if !element.is_valid() {
                return true;
            }
            let mut rotations = [Rotation::Clockwise90, Rotation::Clockwise180, Rotation::CounterClockwise90, Rotation::None];
            shuffle(&mut rotations, self.random);
            for rotation in rotations {
                let mut candidates = element.jigsaws((0, 0, 0), rotation);
                shuffle(&mut candidates, self.random);
                for candidate in &candidates {
                    if !jigsaw.can_attach(candidate) {
                        continue;
                    }
                    let mut origin = (target.0 - candidate.pos.0, target.1 - candidate.pos.1, target.2 - candidate.pos.2);
                    let mut bb = element.bbox(origin, rotation);
                    let terrain = source.element.projection() == Projection::TerrainMatching || element.projection() == Projection::TerrainMatching;
                    let base = if terrain { surface } else { src_bb.y0 };
                    let y0 = jigsaw.pos.1 - (src_bb.y0 + candidate.pos.1) + base + jigsaw.front.offset().1;
                    let dy = y0 - bb.y0;
                    bb.move_y(dy);
                    origin.1 += dy;
                    let fits = match self.variant {
                        Variant::Legacy => self.fits_legacy(&bb, &src_bb, is_inside, inside),
                        Variant::Queued => {
                            if is_inside {
                                space += 1;
                                self.allow(space, src_bb);
                            }
                            self.fits_queued(&bb, space)
                        }
                    };
                    if !fits {
                        continue;
                    }
                    let child = self.pieces.len();
                    self.pieces.push(Piece::new(element, origin, rotation, source.depth + 1));
                    match self.variant {
                        Variant::Legacy => self.expand(child, 0),
                        Variant::Queued => self.enqueue(0, child, space),
                    }
                    return true;
                }
            }
        }
        false
    }

    /// Legacy room test, x and z only: inside the source piece and clear of what attached inside
    /// it, or clear of every piece.
    fn fits_legacy(&self, bb: &BoundingBox, src: &BoundingBox, is_inside: bool, inside: &mut Vec<BoundingBox>) -> bool {
        let overlaps = |o: &BoundingBox| bb.x1 >= o.x0 && bb.x0 <= o.x1 && bb.z1 >= o.z0 && bb.z0 <= o.z1;
        if is_inside {
            let within = src.x0 <= bb.x0 && bb.x1 <= src.x1 && src.z0 <= bb.z0 && bb.z1 <= src.z1;
            if !within || inside.iter().any(overlaps) {
                return false;
            }
            inside.push(*bb);
            true
        } else {
            !self.pieces.iter().any(|p| overlaps(&p.bb))
        }
    }

    fn allow(&mut self, space: usize, bb: BoundingBox) {
        if space < self.spaces.len() {
            self.spaces[space].allowed.push(bb);
        } else {
            self.spaces.push(Space { allowed: vec![bb], occupied: Vec::new() });
        }
    }

    /// Queued room test: wholly inside an allowed box of the space, touching none of its pieces.
    fn fits_queued(&mut self, bb: &BoundingBox, space: usize) -> bool {
        let s = &mut self.spaces[space];
        let within = |a: &BoundingBox| a.x0 <= bb.x0 && bb.x1 <= a.x1 && a.y0 <= bb.y0 && bb.y1 <= a.y1 && a.z0 <= bb.z0 && bb.z1 <= a.z1;
        let touches = |o: &BoundingBox| bb.x1 >= o.x0 && bb.x0 <= o.x1 && bb.z1 >= o.z0 && bb.z0 <= o.z1 && bb.y1 >= o.y0 && bb.y0 <= o.y1;
        if !s.allowed.iter().any(within) || s.occupied.iter().any(touches) {
            return false;
        }
        s.occupied.push(*bb);
        true
    }
}

/// A box side's middle as BDS rounds it.
pub fn middle(a: i32, b: i32) -> i32 {
    a + (b - a + 1).div_euclid(2)
}
