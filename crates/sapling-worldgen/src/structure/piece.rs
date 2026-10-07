//! Legacy `StructurePiece` geometry and block helpers: piece-local coordinates turned by the
//! orientation (`getWorldX/Z` 141f59f30/141f59f60), writes clipped to the chunk
//! (`placeBlock` 141f59f90), boxes (141f5a1b0), random boxes (141f5aa60), half spheres (141f5b3d0).

use super::BoundingBox;
use super::canvas::Canvas;
use crate::block::Block;
use crate::random::Mt19937;

/// Bedrock's legacy directions, as pieces store their orientation.
pub const SOUTH: u8 = 0;
pub const WEST: u8 = 1;
pub const NORTH: u8 = 2;
pub const EAST: u8 = 3;

#[derive(Debug, Clone)]
pub struct Frame {
    pub bb: BoundingBox,
    /// None for pieces in world coordinates (BDS's 0xff).
    pub orientation: Option<u8>,
    pub depth: i32,
    /// Blocks this piece's writes leave alone (`canBeReplaced` false).
    pub keep: fn(Block) -> bool,
}

impl Frame {
    pub fn world_x(&self, x: i32, z: i32) -> i32 {
        match self.orientation {
            Some(WEST) => self.bb.x1 - z,
            Some(EAST) => self.bb.x0 + z,
            Some(_) => self.bb.x0 + x,
            None => x,
        }
    }

    pub fn world_z(&self, x: i32, z: i32) -> i32 {
        match self.orientation {
            Some(SOUTH) => self.bb.z0 + z,
            Some(NORTH) => self.bb.z1 - z,
            Some(_) => self.bb.z0 + x,
            None => z,
        }
    }

    pub fn world_y(&self, y: i32) -> i32 {
        if self.orientation.is_some() { self.bb.y0 + y } else { y }
    }

    pub fn world(&self, x: i32, y: i32, z: i32) -> (i32, i32, i32) {
        (self.world_x(x, z), self.world_y(y), self.world_z(x, z))
    }

    /// The block at piece coordinates, air outside the chunk.
    pub fn get(&self, canvas: &Canvas, x: i32, y: i32, z: i32) -> Block {
        let (wx, wy, wz) = self.world(x, y, z);
        canvas.get_clipped(wx, wy, wz)
    }

    pub fn interior(&self, canvas: &Canvas, x: i32, y: i32, z: i32) -> bool {
        let (wx, wy, wz) = self.world(x, y, z);
        canvas.interior(wx, wy, wz)
    }

    /// `placeBlock`: inside the chunk and over a replaceable block only.
    pub fn place(&self, canvas: &mut Canvas, block: Block, x: i32, y: i32, z: i32) {
        let (wx, wy, wz) = self.world(x, y, z);
        if canvas.bb.contains(wx, wy, wz) && !(self.keep)(canvas.get(wx, wy, wz)) {
            canvas.set(wx, wy, wz, block);
        }
    }

    /// `generateBox`: y, then x, then z; `edge` on the box's faces; `skip_air` leaves air alone.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_box(
        &self,
        canvas: &mut Canvas,
        (x0, y0, z0): (i32, i32, i32),
        (x1, y1, z1): (i32, i32, i32),
        edge: Block,
        fill: Block,
        skip_air: bool,
    ) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    if skip_air && self.get(canvas, x, y, z) == Block::Air {
                        continue;
                    }
                    let on_edge = y == y0 || y == y1 || x == x0 || x == x1 || z == z0 || z == z1;
                    self.place(canvas, if on_edge { edge } else { fill }, x, y, z);
                }
            }
        }
    }

    /// `generateMaybeBox`: a float drawn for every cell, placed when it is at most `chance`;
    /// `interior_only` also needs the cell above below the heightmap.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_maybe_box(
        &self,
        canvas: &mut Canvas,
        random: &mut Mt19937,
        chance: f32,
        (x0, y0, z0): (i32, i32, i32),
        (x1, y1, z1): (i32, i32, i32),
        (edge, fill): (Block, Block),
        skip_air: bool,
        interior_only: bool,
    ) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    if random.next_float() > chance
                        || skip_air && self.get(canvas, x, y, z) == Block::Air
                        || interior_only && !self.interior(canvas, x, y + 1, z)
                    {
                        continue;
                    }
                    let on_edge = y == y0 || y == y1 || x == x0 || x == x1 || z == z0 || z == z1;
                    self.place(canvas, if on_edge { edge } else { fill }, x, y, z);
                }
            }
        }
    }

    /// `generateUpperHalfSphere`: cells within an f32 ellipsoid of the box's lower half.
    pub fn upper_half_sphere(&self, canvas: &mut Canvas, (x0, y0, z0): (i32, i32, i32), (x1, y1, z1): (i32, i32, i32), fill: Block) {
        let height = (y1 - y0 + 1) as f32;
        let (rx, rz) = ((x1 - x0 + 1) as f32 * 0.5, (z1 - z0 + 1) as f32 * 0.5);
        for y in y0..=y1 {
            let fy = (y - y0) as f32 / height;
            for x in x0..=x1 {
                let fx = (x as f32 - (x0 as f32 + rx)) / rx;
                for z in z0..=z1 {
                    let fz = (z as f32 - (z0 as f32 + rz)) / rz;
                    if fz * fz + fx * fx + fy * fy <= 1.05 {
                        self.place(canvas, fill, x, y, z);
                    }
                }
            }
        }
    }

    /// `maybeGenerateBlockIfNotFloating` (141f5aff0): `nextFloat() < chance`, then at least two
    /// of the six neighbours inside the chunk not air.
    #[allow(clippy::too_many_arguments)]
    pub fn maybe_block_if_not_floating(&self, canvas: &mut Canvas, random: &mut Mt19937, chance: f32, (x, y, z): (i32, i32, i32), block: Block) {
        if random.next_float() >= chance {
            return;
        }
        let (wx, wy, wz) = self.world(x, y, z);
        let around = [(0, -1, 0), (0, 1, 0), (0, 0, -1), (0, 0, 1), (-1, 0, 0), (1, 0, 0)];
        let solid = around.iter().filter(|(dx, dy, dz)| canvas.get_clipped(wx + dx, wy + dy, wz + dz) != Block::Air).count();
        if solid >= 2 {
            self.place(canvas, block, x, y, z);
        }
    }
}
