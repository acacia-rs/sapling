//! BDS's three `JigsawPlacement`s. Legacy (villages: ctor 141ee6920, addPieces 141ee6a70, expand
//! 141ee7000, candidates 141ee7770) grows depth first and only keeps boxes from overlapping in x and
//! z. Queued (ancient cities: addPieces 141edde60, expand 141ee4b20, candidates 141ee5600) grows
//! breadth first through a (placement priority, insertion) queue and keeps pieces inside allowed
//! boxes and off occupied ones, a fresh space per jigsaw that attaches inside its piece. V2 (the
//! data-driven structures: `br::worldgen::JigsawAssembler::construct` 144ce64d0, expand lambda
//! 144d11120) is Java's: Java's Random and shuffle, main and fallback pools in one list, one space
//! per piece for what attaches inside it, and the jigsaw's placement priority on the queue.

use std::collections::BinaryHeap;

use super::super::BoundingBox;
use super::super::template::Rotation;
use super::draw::JigsawRandom;
use super::element::{Element, JigsawInfo, Projection};
use super::{Data, Piece, Pool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Legacy,
    Queued,
    V2,
}

#[derive(Default)]
struct Space {
    allowed: Vec<BoundingBox>,
    occupied: Vec<BoundingBox>,
}

pub struct Assembler<'a, R: JigsawRandom> {
    pub variant: Variant,
    pub max_depth: i32,
    pub data: &'static Data,
    pub random: &'a mut R,
    /// The piece class's height at a jigsaw (`PoolElementStructurePiece` vt+0x68).
    pub height: &'a dyn Fn(i32, i32) -> i32,
    /// The structure's pool aliases (v2; the hardcoded pools alias per element).
    pub aliases: &'static [(String, String)],
    pub pieces: Vec<Piece>,
    spaces: Vec<Space>,
    /// (placement priority, -insertion, piece, space).
    queue: BinaryHeap<(i32, i64, usize, usize)>,
    pushed: i64,
}

impl<'a, R: JigsawRandom> Assembler<'a, R> {
    pub fn new(variant: Variant, max_depth: i32, random: &'a mut R, height: &'a dyn Fn(i32, i32) -> i32) -> Self {
        let data = super::data();
        Self { variant, max_depth, data, random, height, aliases: &[], pieces: Vec::new(), spaces: Vec::new(), queue: BinaryHeap::new(), pushed: 0 }
    }

    /// The queued placer's start (141edde60): the start element turned so its `anchor` jigsaw
    /// lands on `start`'s column, `height(centre) - 1` below the anchor, within `max_distance`.
    pub fn start_queued(&mut self, element: &'static Element, start: (i32, i32, i32), rotation: Rotation, anchor: &str, max_distance: i32) {
        let mut jigsaws = element.jigsaws(start, rotation);
        self.random.shuffle(&mut jigsaws);
        let (dx, dy, dz) = jigsaws.iter().find(|j| j.name == anchor).map_or((0, 0, 0), |j| (start.0 - j.pos.0, j.pos.1 - start.1, start.2 - j.pos.2));
        let origin = (start.0 + dx, 0, start.2 + dz);
        let bb = element.bbox(origin, rotation);
        let h = (self.height)(middle(bb.x0, bb.x1), middle(bb.z0, bb.z1));
        let mut piece = Piece::new(element, origin, rotation, 0);
        piece.shift_y(h - dy - bb.y0 - 1);
        let b = piece.bb;
        let (cx, cy, cz) = (middle(b.x0, b.x1), middle(b.y0, b.y1), middle(b.z0, b.z1));
        let limit = BoundingBox::new(cx - max_distance, cy - max_distance, cz - max_distance, cx + max_distance, cy + max_distance, cz + max_distance);
        self.run_queue(piece, limit);
    }

    /// The v2 start: `piece` already at its height, the space `limit` less the piece.
    pub fn run_queue(&mut self, piece: Piece, limit: BoundingBox) {
        self.spaces.push(Space { allowed: vec![limit], occupied: vec![piece.bb] });
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
        self.random.shuffle(&mut jigsaws);
        if self.variant != Variant::Legacy {
            jigsaws.sort_by_key(|j| std::cmp::Reverse(j.selection_priority));
        }
        let mut inside = Vec::new();
        let mut own_space = None;
        for jigsaw in &jigsaws {
            let name = jigsaw.pool.trim_start_matches("minecraft:");
            let name = self.aliases.iter().find(|(from, _)| from == name).map_or(name, |(_, to)| to.as_str());
            let Some(pool) = self.data.pool(name).filter(|p| !p.list.is_empty()) else { continue };
            let fallback = pool.fallback.as_deref().and_then(|f| self.data.pool(f)).filter(|f| !f.list.is_empty());
            if self.variant == Variant::V2 {
                let mut list = Vec::new();
                for p in [(piece.depth != self.max_depth).then_some(pool), fallback].into_iter().flatten() {
                    let mut shuffled = p.list.clone();
                    self.random.shuffle(&mut shuffled);
                    list.extend(shuffled);
                }
                self.try_elements(&piece, jigsaw, &list, space, &mut inside, &mut own_space);
            } else if piece.depth == self.max_depth {
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

    fn try_pool(&mut self, source: &Piece, jigsaw: &JigsawInfo, pool: &Pool, space: usize, inside: &mut Vec<BoundingBox>) -> bool {
        let mut order = pool.list.clone();
        self.random.shuffle(&mut order);
        self.try_elements(source, jigsaw, &order, space, inside, &mut None)
    }

    /// Tries `elements` in order, each in shuffled rotations, each of their shuffled jigsaws that
    /// fits `jigsaw`; true once one is placed or an empty element ends the list.
    fn try_elements(
        &mut self,
        source: &Piece,
        jigsaw: &JigsawInfo,
        elements: &[&'static Element],
        space: usize,
        inside: &mut Vec<BoundingBox>,
        own_space: &mut Option<usize>,
    ) -> bool {
        let src_bb = source.bb;
        let (o, f) = (jigsaw.front.offset(), jigsaw.pos);
        let target = (f.0 + o.0, f.1 + o.1, f.2 + o.2);
        let is_inside = src_bb.contains(target.0, target.1, target.2);
        let mut space = space;
        if self.variant == Variant::V2 && is_inside {
            space = *own_space.get_or_insert_with(|| {
                self.spaces.push(Space { allowed: vec![src_bb], occupied: Vec::new() });
                self.spaces.len() - 1
            });
        }
        for &element in elements {
            if !element.is_valid() {
                return true;
            }
            let mut rotations = match self.variant {
                Variant::V2 => Rotation::ALL,
                _ => [Rotation::Clockwise90, Rotation::Clockwise180, Rotation::CounterClockwise90, Rotation::None],
            };
            self.random.shuffle(&mut rotations);
            for rotation in rotations {
                let mut candidates = element.jigsaws((0, 0, 0), rotation);
                self.random.shuffle(&mut candidates);
                if self.variant == Variant::V2 {
                    candidates.sort_by_key(|j| std::cmp::Reverse(j.selection_priority));
                }
                for candidate in &candidates {
                    if !jigsaw.can_attach(candidate) {
                        continue;
                    }
                    let mut origin = (target.0 - candidate.pos.0, target.1 - candidate.pos.1, target.2 - candidate.pos.2);
                    let mut bb = element.bbox(origin, rotation);
                    let y0 = self.child_y(source, jigsaw, element, candidate);
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
                        Variant::V2 => self.fits_queued(&bb, space),
                    };
                    if !fits {
                        continue;
                    }
                    let child = self.pieces.len();
                    self.pieces.push(Piece::new(element, origin, rotation, source.depth + 1));
                    match self.variant {
                        Variant::Legacy => self.expand(child, 0),
                        Variant::Queued => self.enqueue(0, child, space),
                        Variant::V2 => self.enqueue(jigsaw.placement_priority, child, space),
                    }
                    return true;
                }
            }
        }
        false
    }

    /// The child's bottom: rigid on rigid keeps the jigsaws face to face; otherwise the hardcoded
    /// placers stand it on the surface under the jigsaw, v2 the candidate jigsaw on it.
    fn child_y(&self, source: &Piece, jigsaw: &JigsawInfo, element: &Element, candidate: &JigsawInfo) -> i32 {
        let rigid = source.element.projection() == Projection::Rigid && element.projection() == Projection::Rigid;
        let surface = (self.height)(jigsaw.pos.0, jigsaw.pos.2);
        let step = jigsaw.front.offset().1;
        match (rigid, self.variant) {
            (true, _) => jigsaw.pos.1 - candidate.pos.1 + step,
            (false, Variant::V2) => surface - candidate.pos.1,
            (false, _) => jigsaw.pos.1 - (source.bb.y0 + candidate.pos.1) + surface - 1 + step,
        }
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
