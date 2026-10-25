//! End cities (`EndCityFeature`, vtable 14a926270): Java 1.12's `MapGenEndCity` and `EndCityPieces` on MT19937, the
//! only structure of the End (docs/research/worldgen-bds-end.md, "End cities").

mod pieces;

use super::BoundingBox;
use super::canvas::Canvas;
use super::placement::{Grid, Spread};
use super::template::{Placement, Rotation, Template, named};
use crate::block::Block;
use crate::block_entity::{BlockEntity, cell_seed};
use crate::chunk::ChunkShape;
use crate::random::Mt19937;

pub use pieces::Piece;

pub const GRID: Grid = Grid::new(20, 11, 10_387_313, Spread::Triangular);
/// The farthest any piece lies from its start chunk, in chunks (bridges and a ship 70 blocks out at depth 8).
pub const REACH: i32 = 12;
const MIN_HEIGHT: i32 = 60;

#[derive(Debug)]
pub struct EndCity {
    pub start: (i32, i32),
    pub pieces: Vec<Piece>,
    pub bb: BoundingBox,
}

/// The start's turn: MT(x + z * 10387313)'s first draw (getYPosForStructure 1483134a0, createStructureStart).
fn rotation((x, z): (i32, i32)) -> Rotation {
    Rotation::from_index((Mt19937::new(x.wrapping_add(z.wrapping_mul(10_387_313)) as u32).next_u32() & 3) as i32)
}

/// `getYPosForStructure` (1483134a0): the least of one above the top block at the chunk's (7, 7) and three corners
/// 5 blocks off it toward the start's turn, on the chunk's raw terrain.
pub fn start_height(terrain: &ChunkShape, chunk: (i32, i32)) -> i32 {
    const DX: [usize; 4] = [12, 2, 2, 12];
    const DZ: [usize; 4] = [12, 12, 2, 2];
    let r = rotation(chunk) as usize;
    let top = |x: usize, z: usize| (0..crate::end::HEIGHT).rev().find(|&y| terrain.block(x, y, z) != Block::Air).map_or(0, |y| y + 1);
    let (h1, h2, h3, h4) = (top(7, 7), top(7, DZ[r]), top(DX[r], 7), top(DX[r], DZ[r]));
    h3.min(h4).min(h1.min(h2))
}

/// `isFeatureChunk` (148313230) past the grid: beyond 64 chunks of the origin, on an island whose height is not
/// negative (Java asks more than 40), standing at least 60 high.
pub fn is_start(chunk: (i32, i32), island_height: f32, terrain: &ChunkShape) -> bool {
    let far = i64::from(chunk.0) * i64::from(chunk.0) + i64::from(chunk.1) * i64::from(chunk.1) > 4096;
    far && island_height.floor() as i16 >= 0 && start_height(terrain, chunk) >= MIN_HEIGHT
}

/// The city of start chunk `chunk` (`createStructureStart` 148313a10); `random` is the grid's generator after its draws.
pub fn generate(random: &mut Mt19937, chunk: (i32, i32), terrain: &ChunkShape) -> EndCity {
    let origin = (chunk.0 * 16 + 8, start_height(terrain, chunk), chunk.1 * 16 + 8);
    let pieces = pieces::generate(random, origin, rotation(chunk));
    let bb = pieces.iter().skip(1).fold(pieces[0].bb, |bb, p| bb.union(&p.bb));
    EndCity { start: chunk, pieces, bb }
}

impl EndCity {
    /// Every piece meeting `canvas`'s chunk, in list order, clipped to it (`TemplateStructurePiece::postProcess`
    /// 141f797b0). Chests' loot seeds are the only draws, and nothing else reads them.
    pub fn post_process(&self, canvas: &mut Canvas) {
        let bb = canvas.bb;
        for piece in self.pieces.iter().filter(|p| p.bb.intersects(&bb)) {
            let template = Template::get(&format!("endcity/{}", piece.name));
            let mut placement = Placement::new(piece.rotation, bb);
            if !piece.overwrite {
                placement.ignore = Some("air");
            }
            placement.place(canvas, template, piece.pos);
            // "Chest" and "Sentry" markers write nothing; their cells keep what was there. A chest marker's chest is below it.
            for (index, marker) in &template.markers {
                let (dx, dy, dz) = piece.rotation.apply(template.blocks[*index].pos);
                let (x, y, z) = (piece.pos.0 + dx, piece.pos.1 + dy, piece.pos.2 + dz);
                if marker == "Chest" && canvas.get(x, y - 1, z).name() == "chest" {
                    canvas.set_entity(x, y - 1, z, BlockEntity::Loot { table: "loot_tables/chests/end_city_treasure.json", seed: cell_seed([x, y - 1, z]) });
                } else if marker == "Elytra" && bb.contains(x, y, z) {
                    let facing = [3, 4, 2, 5][piece.rotation as usize];
                    canvas.set(x, y, z, named("frame", &format!("facing_direction={facing},item_frame_map_bit=0,item_frame_photo_bit=0"), false));
                }
            }
        }
    }
}
