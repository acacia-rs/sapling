//! The Nether's terrain as `NetherGenerator::prepareHeights` (1481e9e90) fills a chunk: Java 1.12's hell
//! density (`ChunkGeneratorHell`) in f32 on a 5x17x5 cell grid, from octaves the constructor (1481e6d60)
//! draws off one MT seeded with the 32-bit seed. Stone where positive, water below the sea level 32, air above, as
//! BDS writes them: the surface builders turn the stone into netherrack and the water into lava.

use crate::block::Block;
use crate::chunk::ChunkShape;
use crate::noise::{ImprovedNoise, SimplexNoise, fill_grid, legacy_octaves, legacy_simplex_octaves};
use crate::random::Mt19937;

/// Cell corners per column: 16 cells of 8 blocks, 0..=128.
pub const CELL_ROWS: usize = 17;
pub const HEIGHT: i32 = 128;
pub const SEA_LEVEL: i32 = 32;
/// Draws one octave's constructor takes: three offsets and the permutation's 256.
const OCTAVE_DRAWS: u32 = 3 + 256;

pub struct NetherNoises {
    min_limit: Vec<ImprovedNoise>,
    max_limit: Vec<ImprovedNoise>,
    main: Vec<ImprovedNoise>,
    /// +0x198, summed by `loadChunk` into the surface builders' depth buffer.
    pub depth: Vec<SimplexNoise>,
    /// +0x1a0, the surface material adjustments' noise.
    pub adjustment: Vec<SimplexNoise>,
}

impl NetherNoises {
    /// The constructor's eight noises in its order; the ones no ported stage reads only take their draws:
    /// +0x180 (4 octaves) and Java's scale (10) and depth (16) noises, which the hell density fills and never uses.
    pub fn new(seed: u32) -> Self {
        let mut random = Mt19937::new(seed);
        let min_limit = legacy_octaves(&mut random, 16);
        let max_limit = legacy_octaves(&mut random, 16);
        let main = legacy_octaves(&mut random, 8);
        random.consume_count(4 * OCTAVE_DRAWS);
        let depth = legacy_simplex_octaves(&mut random, 4);
        random.consume_count(26 * OCTAVE_DRAWS);
        let adjustment = legacy_simplex_octaves(&mut random, 3);
        Self { min_limit, max_limit, main, depth, adjustment }
    }

    /// Every cell corner's density, `[x][z][y]` (x, z 0..5, y 0..17).
    pub fn cells(&self, chunk_x: i32, chunk_z: i32) -> Vec<f32> {
        let origin = [(chunk_x * 4) as f32, 0.0, (chunk_z * 4) as f32];
        let main = fill_grid::<CELL_ROWS>(&self.main, origin, [f32::from_bits(0x4108_e1e5), f32::from_bits(0x4208_e1e4)]);
        let xz = f32::from_bits(0x442b_1a5e);
        let min = fill_grid::<CELL_ROWS>(&self.min_limit, origin, [xz, f32::from_bits(0x4500_53c6)]);
        let max = fill_grid::<CELL_ROWS>(&self.max_limit, origin, [xz, f32::from_bits(0x4500_53c6)]);
        let offsets = y_offsets();
        (0..main.len())
            .map(|i| {
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
                top_slide(blended - offsets[i % CELL_ROWS], i % CELL_ROWS)
            })
            .collect()
    }

    /// The chunk's stone, water and air, interpolated z, x, then y per cell as BDS does.
    pub fn chunk_shape(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let cells = self.cells(chunk_x, chunk_z);
        let at = |x: usize, z: usize, y: usize| cells[(x * 5 + z) * CELL_ROWS + y];
        let mut shape = ChunkShape::empty();
        for cx in 0..4 {
            for cz in 0..4 {
                for cy in 0..CELL_ROWS - 1 {
                    let (d000, d100, d010, d110) = (at(cx, cz, cy), at(cx + 1, cz, cy), at(cx, cz + 1, cy), at(cx + 1, cz + 1, cy));
                    let (d001, d101, d011, d111) = (at(cx, cz, cy + 1), at(cx + 1, cz, cy + 1), at(cx, cz + 1, cy + 1), at(cx + 1, cz + 1, cy + 1));
                    for dz in 0..4 {
                        let tz = dz as f32 * 0.25;
                        let x0_lower = (d010 - d000) * tz + d000;
                        let x0_upper = (d011 - d001) * tz + d001;
                        for dx in 0..4 {
                            let tx = dx as f32 * 0.25;
                            let lower = ((d110 - d100) * tz + d100 - x0_lower) * tx + x0_lower;
                            let step = (tx * ((d111 - d101) * tz + d101 - x0_upper) + x0_upper) - lower;
                            for dy in 0..8 {
                                let y = (cy * 8 + dy) as i32;
                                let block = if step * (dy as f32 * 0.125) + lower > 0.0 {
                                    Block::Stone
                                } else if y < SEA_LEVEL {
                                    Block::Water
                                } else {
                                    Block::Air
                                };
                                shape.set_block(cx * 4 + dx, y, cz * 4 + dz, block);
                            }
                        }
                    }
                }
            }
        }
        shape
    }
}

/// Java's per-row offset: `2cos(6pi y/17)`, minus `10(4-t)^3` within 4 rows of either end, through Bedrock's
/// sine table.
fn y_offsets() -> [f32; CELL_ROWS] {
    std::array::from_fn(|y| {
        let row = y as f32;
        let edge = if y > 8 { 16.0 - row } else { row };
        let mut offset = crate::mth::cos(row * std::f32::consts::PI * 6.0 / 17.0);
        offset = offset + offset;
        if edge < 4.0 {
            let d = 4.0 - edge;
            offset += d * d * d * -10.0;
        }
        offset
    })
}

/// The top three rows fade to -10 (vector constants 14a923400/14a923410).
fn top_slide(density: f32, row: usize) -> f32 {
    match row {
        14 => density * f32::from_bits(0x3f2a_aaaa) + f32::from_bits(0xc055_5556),
        15 => density * f32::from_bits(0x3eaa_aaaa) + f32::from_bits(0xc0d5_5556),
        16 => density * 0.0 + -10.0,
        _ => density,
    }
}
