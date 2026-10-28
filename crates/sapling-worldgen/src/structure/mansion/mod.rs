//! Woodland mansions (`WoodlandMansionFeature`: start 1480a8c20, start postProcess 1480ab380),
//! BDS's port of Java's 1.11 mansion on its MT: a rotation, the start at the chunk's middle on the
//! lowest of four raw terrain heights, the floor plans (`grid`), and the template pieces they make
//! (`placer`, `rooms`, `roof`), placed as `TemplateStructurePiece`s (`piece`).

mod dir;
mod grid;
mod piece;
mod placer;
mod roof;
mod rooms;

use std::sync::Arc;

use super::canvas::Canvas;
use super::probe::RawHeights;
use super::template::{Rotation, Template};
use super::{BoundingBox, StructureKind, Structures};
use crate::density::Density;
use crate::random::Mt19937;

pub type Pos = (i32, i32, i32);

/// Mansions reach about 90 blocks from their start.
const REACH: i32 = 7;

/// Java's mirrors as BDS numbers them (1 flips template z, 2 template x).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirror {
    None,
    LeftRight,
    FrontBack,
}

pub struct Piece {
    pub template: &'static Template,
    pub origin: Pos,
    pub rotation: Rotation,
    pub mirror: Mirror,
    pub bb: BoundingBox,
}

impl Piece {
    fn new(name: String, origin: Pos, rotation: Rotation, mirror: Mirror) -> Self {
        let template = Template::find(&format!("mansion/{name}")).unwrap_or_else(|| panic!("mansion template {name}"));
        let (sx, sy, sz) = template.size;
        let a = piece::transform((0, 0, 0), rotation, mirror);
        let b = piece::transform((sx - 1, sy - 1, sz - 1), rotation, mirror);
        let (x, y, z) = origin;
        let bb = BoundingBox::new(x + a.0.min(b.0), y, z + a.2.min(b.2), x + a.0.max(b.0), y + sy - 1, z + a.2.max(b.2));
        Self { template, origin, rotation, mirror, bb }
    }
}

pub struct Mansion {
    pub pieces: Vec<Piece>,
    pub origin: Pos,
    pub rotation: Rotation,
    pub bb: BoundingBox,
}

pub fn generate(density: &Density, random: &mut Mt19937, chunk: (i32, i32)) -> Mansion {
    let rotation = Rotation::from_index(random.next_int(4));
    let (ox, oz) = match rotation {
        Rotation::None => (12, 12),
        Rotation::Clockwise90 => (2, 12),
        Rotation::Clockwise180 => (2, 2),
        Rotation::CounterClockwise90 => (12, 2),
    };
    let raw = RawHeights::new(density);
    let (bx, bz) = (chunk.0 * 16, chunk.1 * 16);
    let lowest = [(7, 7), (7, oz), (ox, 7), (ox, oz)].iter().map(|&(x, z)| raw.at(bx + x, bz + z)).min().unwrap();
    // 141a2ac40 gives the first free height; the start sits one above it.
    let origin = (bx + 8, lowest + 1, bz + 8);
    let grid = grid::MansionGrid::new(random);
    let mut placer = placer::Placer::new(random, &grid);
    placer.create(origin, rotation);
    let pieces = placer.pieces;
    let bb = pieces.iter().skip(1).fold(pieces[0].bb, |b, p| b.union(&p.bb));
    Mansion { pieces, origin, rotation, bb }
}

impl Structures {
    /// Mansions with a piece meeting `chunk`.
    pub(super) fn mansions_near(&self, density: &Density, chunk: (i32, i32)) -> Vec<Arc<Mansion>> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let starts = self.starts_near(density, StructureKind::Mansion, chunk, REACH);
        starts.into_iter().map(|start| self.mansion(density, start)).filter(|m| m.bb.intersects(&column)).collect()
    }

    /// The mansion starting in `start`, whatever its start check.
    pub(super) fn mansion(&self, density: &Density, start: (i32, i32)) -> Arc<Mansion> {
        let grid = StructureKind::Mansion.grid().unwrap();
        self.templates.mansions.get(start, || generate(density, &mut grid.candidate_random(self.level_seed, start).1, start))
    }

    /// `seed` is the chunk's structure seed: the mansions' pieces share one generator of it.
    pub(super) fn place_mansions(canvas: &mut Canvas, mansions: &[Arc<Mansion>], seed: u32) {
        let mut random = Mt19937::new(seed);
        for mansion in mansions {
            mansion.post_process(canvas, &mut random);
        }
    }
}
