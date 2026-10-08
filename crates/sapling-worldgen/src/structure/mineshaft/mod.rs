//! Mineshafts: BDS's port of Java's legacy mineshaft on MT19937 (`MineshaftFeature::
//! createStructureStart` 14831b270, piece factory `MineshaftPiece::generateAndAddPiece`
//! 14831bf70), new-world variant (1.18+ heights, chains, floors under air).

mod corridor;
mod rooms;

use super::BoundingBox;
use super::canvas::Canvas;
use super::piece::{Frame, NORTH, SOUTH, WEST};
use super::placement::MineshaftSeed;
use crate::biome::ChunkBiomes;
use crate::block::Block;
use crate::random::Mt19937;
use corridor::Corridor;
pub use corridor::full_solid;

/// Badlands biomes (tag `mesa`): dark oak mineshafts, half the room children.
pub const MESA_BIOMES: &[u32] = &[37, 38, 39, 165, 166, 167];
const DEEP_DARK: u32 = 190;
const SEA_LEVEL: i32 = 63;
const MIN_Y: i32 = -64;

#[derive(Debug, Clone, Copy)]
pub struct Materials {
    pub dark: bool,
    pub room_chance: f32,
}

impl Materials {
    pub fn planks(self) -> Block {
        if self.dark { Block::DarkOakPlanks } else { Block::OakPlanks }
    }

    pub fn log(self) -> Block {
        if self.dark { Block::DarkOakLog } else { Block::OakLog }
    }

    /// A fence joined to the solid blocks beside it (east 1, north 2, south 4, west 8).
    pub fn fence(self, bits: u8) -> Block {
        let first = if self.dark { Block::DarkOakFence0 } else { Block::OakFence0 };
        Block::ALL[first.index() + bits as usize]
    }

    pub fn is_fence(self, block: Block) -> bool {
        if self.dark {
            (Block::DarkOakFence0..=Block::DarkOakFence15).contains(&block)
        } else {
            (Block::OakFence0..=Block::OakFence15).contains(&block)
        }
    }
}

#[derive(Debug, Clone)]
pub enum Kind {
    /// Entrance boxes of the children, carved out of the room's walls.
    Room(Vec<BoundingBox>),
    Corridor(Corridor),
    Crossing {
        direction: u8,
        two_floors: bool,
    },
    Stairs,
}

#[derive(Debug, Clone)]
pub struct Piece {
    pub frame: Frame,
    pub kind: Kind,
}

#[derive(Debug, Clone)]
pub struct Mineshaft {
    pub pieces: Vec<Piece>,
    pub materials: Materials,
    pub bb: BoundingBox,
}

/// `canBeReplaced` (14831c830): pieces never overwrite their own wood, chains or bedrock.
fn kept(block: Block) -> bool {
    matches!(block, Block::OakPlanks | Block::DarkOakPlanks | Block::OakLog | Block::DarkOakLog | Block::IronChain | Block::Bedrock)
        || (Block::OakFence0..=Block::DarkOakFence15).contains(&block)
}

pub(super) fn frame(bb: BoundingBox, orientation: Option<u8>, depth: i32) -> Frame {
    Frame { bb, orientation, depth, keep: kept }
}

/// A start's pieces; `mesa` from the point biome at the chunk corner, y 0; `surface` is the
/// preliminary surface at a quart column.
pub fn generate(seed: &MineshaftSeed, chunk: (i32, i32), mesa: bool, surface: &dyn Fn(i32, i32) -> i32) -> Mineshaft {
    let (_, mut random) = seed.random(chunk);
    let materials = Materials { dark: mesa, room_chance: if mesa { 0.5 } else { 1.0 } };
    let (x, z) = (chunk.0 * 16, chunk.1 * 16);
    // C++ evaluates the box's arguments last to first.
    let dz = random.next_int(6);
    let dy = random.next_int(6);
    let dx = random.next_int(6);
    let room = BoundingBox::new(x + 2, 50, z + 2, x + 9 + dx, 54 + dy, z + 9 + dz);
    let mut builder = Builder { pieces: vec![Piece { frame: frame(room, None, 0), kind: Kind::Room(Vec::new()) }], materials, start: room };
    rooms::room_children(&mut builder, 0, &mut random);
    let mut bb = builder.pieces[0].frame.bb;
    for piece in &builder.pieces {
        bb = bb.union(&piece.frame.bb);
    }
    let offset = if mesa {
        let centre = (bb.x0 + (bb.x1 - bb.x0 + 1) / 2, bb.z0 + (bb.z1 - bb.z0 + 1) / 2);
        let top = surface(centre.0 >> 2, centre.1 >> 2);
        let y = if SEA_LEVEL < top { SEA_LEVEL + random.next_int(top - SEA_LEVEL + 1) } else { SEA_LEVEL };
        y - (bb.y0 + (bb.y1 - bb.y0 + 1) / 2)
    } else {
        let mut y = MIN_Y + (bb.y1 - bb.y0) + 2;
        if y < SEA_LEVEL - 10 {
            y += random.next_int(SEA_LEVEL - 10 - y);
        }
        y - bb.y1
    };
    for piece in &mut builder.pieces {
        piece.frame.bb.move_y(offset);
        if let Kind::Room(entrances) = &mut piece.kind {
            entrances.iter_mut().for_each(|e| e.move_y(offset));
        }
    }
    bb.move_y(offset);
    Mineshaft { pieces: builder.pieces, materials, bb }
}

pub(super) struct Builder {
    pub pieces: Vec<Piece>,
    pub materials: Materials,
    pub start: BoundingBox,
}

impl Builder {
    fn collides(&self, bb: &BoundingBox) -> bool {
        self.pieces.iter().any(|p| p.frame.bb.intersects(bb))
    }

    /// `generateAndAddPiece`: a crossing (20%), stairs (10%) or corridor at the foot position,
    /// then its own children, depth first.
    pub fn add(&mut self, random: &mut Mt19937, (x, y, z): (i32, i32, i32), direction: u8, depth: i32) {
        if depth > 8 || (x - self.start.x0).abs() > 80 || (z - self.start.z0).abs() > 80 {
            return;
        }
        let depth = depth + 1;
        let roll = random.next_int(100);
        let piece = if roll >= 80 {
            let two_floors = random.next_int(4) == 0;
            let top = y + 2 + if two_floors { 4 } else { 0 };
            let bb = match direction {
                SOUTH => BoundingBox::new(x - 1, y, z, x + 3, top, z + 4),
                WEST => BoundingBox::new(x - 4, y, z - 1, x, top, z + 3),
                NORTH => BoundingBox::new(x - 1, y, z - 4, x + 3, top, z),
                _ => BoundingBox::new(x, y, z - 1, x + 4, top, z + 3),
            };
            (!self.collides(&bb)).then(|| Piece { frame: frame(bb, None, depth), kind: Kind::Crossing { direction, two_floors } })
        } else if roll >= 70 {
            let bb = match direction {
                SOUTH => BoundingBox::new(x, y - 5, z, x + 2, y + 2, z + 8),
                WEST => BoundingBox::new(x - 8, y - 5, z, x, y + 2, z + 2),
                NORTH => BoundingBox::new(x, y - 5, z - 8, x + 2, y + 2, z),
                _ => BoundingBox::new(x, y - 5, z, x + 8, y + 2, z + 2),
            };
            (!self.collides(&bb)).then(|| Piece { frame: frame(bb, Some(direction), depth), kind: Kind::Stairs })
        } else {
            Corridor::find(self, random, (x, y, z), direction, depth)
        };
        let Some(piece) = piece else { return };
        let index = self.pieces.len();
        self.pieces.push(piece);
        match self.pieces[index].kind {
            Kind::Corridor(_) => corridor::children(self, index, random),
            Kind::Crossing { .. } => rooms::crossing_children(self, index, random),
            Kind::Stairs => rooms::stairs_children(self, index, random),
            Kind::Room(_) => {}
        }
    }
}

impl Mineshaft {
    /// The pieces reaching the chunk, in start order, on the chunk's structure generator.
    pub fn post_process(&self, canvas: &mut Canvas, random: &mut Mt19937, biomes: &ChunkBiomes) {
        let column = canvas.bb;
        for piece in self.pieces.iter().filter(|p| p.frame.bb.intersects(&column)) {
            if invalid_location(&piece.frame, canvas, biomes) {
                continue;
            }
            match &piece.kind {
                Kind::Room(entrances) => rooms::room(&piece.frame, entrances, canvas),
                Kind::Corridor(c) => c.post_process(&piece.frame, self.materials, canvas, random),
                Kind::Crossing { two_floors, .. } => rooms::crossing(&piece.frame, *two_floors, self.materials, canvas),
                Kind::Stairs => rooms::stairs(&piece.frame, canvas),
            }
        }
    }
}

/// `isInInvalidLocation` (14831dd40, 141f59c80): the box grown by one and clipped to the chunk
/// is deep dark at its centre (chunk biomes), or has a liquid on one of its faces.
fn invalid_location(frame: &Frame, canvas: &Canvas, biomes: &ChunkBiomes) -> bool {
    let (c, b) = (canvas.bb, frame.bb);
    let lo = ((b.x0 - 1).max(c.x0), (b.y0 - 1).max(c.y0), (b.z0 - 1).max(c.z0));
    let hi = ((b.x1 + 1).min(c.x1), (b.y1 + 1).min(c.y1), (b.z1 + 1).min(c.z1));
    let centre = ((lo.0 + hi.0) / 2, (lo.1 + hi.1) / 2, (lo.2 + hi.2) / 2);
    if biomes.biome((centre.0 & 15) as usize, centre.1, (centre.2 & 15) as usize) == DEEP_DARK {
        return true;
    }
    for x in lo.0..=hi.0 {
        for y in lo.1..=hi.1 {
            for z in lo.2..=hi.2 {
                let face = x == lo.0 || x == hi.0 || y == lo.1 || y == hi.1 || z == lo.2 || z == hi.2;
                if face && canvas.get(x, y, z).is_fluid() {
                    return true;
                }
            }
        }
    }
    false
}

/// Fence connections: the solid full blocks and fences beside it, east 1, north 2, south 4, west 8.
pub(super) fn fence_at(materials: Materials, canvas: &Canvas, (x, y, z): (i32, i32, i32)) -> Block {
    let joins = |b: Block| materials.is_fence(b) || corridor::full_solid(b);
    let bits = [(1, 0, 1u8), (0, -1, 2), (0, 1, 4), (-1, 0, 8)].iter().filter(|(dx, dz, _)| joins(canvas.get(x + dx, y, z + dz))).map(|t| t.2).sum();
    materials.fence(bits)
}
