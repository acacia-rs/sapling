//! The End's terrain as `TheEndGenerator` fills a chunk: Java 1.12's `ChunkGeneratorEnd` in f32. The density
//! (1482432f0) blends three octave groups over a 3x33x3 cell grid and adds the island height; the interpolation
//! (148243be0) writes end stone where positive, air elsewhere, y 0..128 (docs/research/worldgen-bds-end.md).

use crate::block::Block;
use crate::chunk::ChunkShape;
use crate::noise::{ImprovedNoise, SimplexNoise, fill_square_grid, legacy_octaves, legacy_simplex_octaves};
use crate::random::Mt19937;

pub const HEIGHT: i32 = 128;
const SIDE: usize = 3;
const COLUMNS: usize = SIDE * SIDE;
const ROWS: usize = 33;

pub struct EndNoises {
    min_limit: Vec<ImprovedNoise>,
    max_limit: Vec<ImprovedNoise>,
    main: Vec<ImprovedNoise>,
    /// +0x180: Java's `islandNoise`, sampled raw (no offsets) at chunk coordinates.
    island: SimplexNoise,
}

impl EndNoises {
    /// The constructor (1482414a0) draws them in this order off one MT on the seed's low 32 bits.
    pub fn new(seed: u32) -> Self {
        let mut random = Mt19937::new(seed);
        let min_limit = legacy_octaves(&mut random, 16);
        let max_limit = legacy_octaves(&mut random, 16);
        let main = legacy_octaves(&mut random, 8);
        let island = legacy_simplex_octaves(&mut random, 1).remove(0);
        Self { min_limit, max_limit, main, island }
    }

    /// The island height of cell column (`cell_x`, `cell_z`) of a chunk: the main island's cone, raised by the outer
    /// islands of chunks beyond 64 chunks of the origin whose noise is under -0.9.
    pub fn island_height(&self, chunk_x: i32, chunk_z: i32, cell_x: i32, cell_z: i32) -> f32 {
        let clamp = |h: f32| if h > 80.0 { 80.0 } else if h <= -100.0 { -100.0 } else { h };
        let (x, z) = ((cell_x + chunk_x * 2) as f32, (cell_z + chunk_z * 2) as f32);
        let mut height = clamp((z * z + x * x).sqrt() * -8.0 + 100.0);
        for dx in -12..=12 {
            let ix = chunk_x + dx;
            let a = (cell_x + dx * -2) as f32;
            for dz in -12..=12 {
                let iz = chunk_z + dz;
                if i64::from(iz) * i64::from(iz) + i64::from(ix) * i64::from(ix) <= 4096 ||self.island.value(ix as f32, iz as f32) >= -0.9 {
                    continue;
                }
                let factor = ((iz.unsigned_abs().wrapping_mul(147)).wrapping_add(ix.unsigned_abs().wrapping_mul(3439)) % 13 + 9) as f32;
                let b = (cell_z - dz * 2) as f32;
                let outer = clamp(100.0 - (b * b + a * a).sqrt() * factor);
                if outer > height {
                    height = outer;
                }
            }
        }
        height
    }

    /// Every cell corner's density, `[x][z][y]` (x, z 0..3, y 0..33).
    pub fn cells(&self, chunk_x: i32, chunk_z: i32) -> Vec<f32> {
        let origin = [(chunk_x * 2) as f32, 0.0, (chunk_z * 2) as f32];
        let main = fill_square_grid::<SIDE, COLUMNS, ROWS>(&self.main, origin, [17.1103, 4.277_575]);
        let min = fill_square_grid::<SIDE, COLUMNS, ROWS>(&self.min_limit, origin, [1368.824, 684.412]);
        let max = fill_square_grid::<SIDE, COLUMNS, ROWS>(&self.max_limit, origin, [1368.824, 684.412]);
        let mut out = vec![0.0f32; COLUMNS * ROWS];
        for column in 0..COLUMNS {
            let height = self.island_height(chunk_x, chunk_z, (column / SIDE) as i32, (column % SIDE) as i32);
            for row in 0..ROWS {
                let i = column * ROWS + row;
                let low = min[i] * 0.001_953_125;
                let high = max[i] * 0.001_953_125;
                let t = (main[i] / 10.0 + 1.0) * 0.5;
                let blended = if t < 0.0 {
                    low
                } else if t <= 1.0 {
                    (high - low) * t + low
                } else {
                    high
                };
                out[i] = slide(row, blended, height);
            }
        }
        out
    }

    pub fn chunk_shape(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let cells = self.cells(chunk_x, chunk_z);
        let mut shape = ChunkShape::empty();
        let at = |x: usize, z: usize, y: usize| cells[(x * SIDE + z) * ROWS + y];
        for cx in 0..2 {
            for cz in 0..2 {
                for cy in 0..32 {
                    let [c000, c001, c100, c101] = [at(cx, cz, cy), at(cx, cz, cy + 1), at(cx + 1, cz, cy), at(cx + 1, cz, cy + 1)];
                    let [c010, c011, c110, c111] = [at(cx, cz + 1, cy), at(cx, cz + 1, cy + 1), at(cx + 1, cz + 1, cy), at(cx + 1, cz + 1, cy + 1)];
                    for lz in 0..8 {
                        let tz = lz as f32 * 0.125;
                        let low = (c010 - c000) * tz + c000;
                        let high = (c011 - c001) * tz + c001;
                        for lx in 0..8 {
                            let tx = lx as f32 * 0.125;
                            let bottom = (((c110 - c100) * tz + c100) - low) * tx + low;
                            let step = (tx * ((tz * (c111 - c101) + c101) - high) + high) - bottom;
                            for (ly, frac) in [0.0f32, 0.25, 0.5, 0.75].into_iter().enumerate() {
                                if step * frac + bottom > 0.0 {
                                    shape.set_block(cx * 8 + lx, (cy * 4 + ly) as i32, cz * 8 + lz, Block::EndStone);
                                }
                            }
                        }
                    }
                }
            }
        }
        shape
    }
}

/// The top slide toward -3000 above row 14, the bottom slide toward -30 below row 8; row 32 is 1482432f0's scalar
/// tail, the top slide folded (s = 18/64), which gives the same value.
fn slide(row: usize, blended: f32, height: f32) -> f32 {
    if row == ROWS - 1 {
        return (height + (blended + -8.0)) * 0.71875 + -843.75;
    }
    let value = blended + -8.0 + height;
    if row > 14 {
        let t = ((row as i32 - 14) as f32 * 0.015625).clamp(0.0, 1.0);
        t * -3000.0 + (1.0 - t) * value
    } else if row < 8 {
        let t = (8 - row as i32) as f32 / 7.0;
        t * -30.0 + (1.0 - t) * value
    } else {
        value
    }
}
