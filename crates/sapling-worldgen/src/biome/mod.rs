//! Per-block overworld biomes as BDS fills them (`BiomeSource3d::fillBiomes` 14805f140): the
//! multi-noise tree sampled on an 8-block lattice (14805eb90), then a jittered nearest-lattice-point
//! zoom to single blocks (148a72d90 offsets, 142f67a10 pick).

mod btree;
mod ids;

pub use btree::{ClimatePoint, Leaf, nearest};
pub use ids::bedrock_id;

use crate::climate::ClimateNoises;
use crate::terrain::{HEIGHT, MIN_Y};

/// Lattice rows: one below the world and one above it (y -72..=320, step 8).
const LATTICE_ROWS: usize = HEIGHT / 8 + 2;
const LATTICE_MIN_Y: i32 = MIN_Y / 8 - 1;
/// A chunk needs lattice columns 2c-1 ..= 2c+2.
const LATTICE_COLUMNS: usize = 4;

/// Java's `Climate.quantizeCoord`, but in f64 as BDS has it (14805fad0).
fn quantize(value: f32) -> i64 {
    (value as f64 * 10000.0) as i64
}

struct LatticePoint {
    biome: u32,
    jitter: [f32; 3],
}

pub struct ChunkBiomes {
    ids: Vec<u32>,
    /// Which lattice point each block took, for diagnosing (lattice x, y, z).
    picks: Vec<[i32; 3]>,
}

fn block_index(x: usize, y: i32, z: usize) -> usize {
    ((y - MIN_Y) as usize * 16 + z) * 16 + x
}

impl ChunkBiomes {
    pub fn biome(&self, x: usize, y: i32, z: usize) -> u32 {
        self.ids[block_index(x, y, z)]
    }

    pub fn lattice_point(&self, x: usize, y: i32, z: usize) -> [i32; 3] {
        self.picks[block_index(x, y, z)]
    }
}

/// The lattice point's offset in [-0.45, 0.45)^3: a `java.util.Random` step seeded by the
/// point's position hash and the low 32 bits of the world seed.
fn jitter(seed: i64, [x, y, z]: [i32; 3]) -> [f32; 3] {
    const MULTIPLIER: u64 = 0x5DEECE66D;
    let hash = crate::random::position_seed(x, y, z) as u64;
    let mut state = hash ^ (seed as u32 as u64 ^ MULTIPLIER);
    std::array::from_fn(|_| {
        state = state.wrapping_mul(MULTIPLIER).wrapping_add(0xB);
        (((state >> 24) & 0xFF_FFFF) as f32 * 5.960_464_5e-8 + -0.5) * 0.9
    })
}

impl ClimateNoises {
    /// One lattice column, bottom up, each search warm-started from the one below.
    fn lattice_column(&self, lx: i32, lz: i32) -> Vec<u32> {
        let climate = self.sample(lx * 2, lz * 2);
        let mut warm = None;
        (0..LATTICE_ROWS as i32)
            .map(|row| {
                let y = (LATTICE_MIN_Y + row) * 8;
                let depth = y as f32 * -0.007_812_5 + 1.0 + climate.offset;
                let point = [climate.temperature, climate.humidity, climate.continentalness, climate.erosion, depth, climate.weirdness].map(quantize);
                let leaf = nearest(&point, warm);
                warm = Some(leaf);
                bedrock_id(leaf.biome()).unwrap_or(u32::MAX)
            })
            .collect()
    }

    pub fn chunk_biomes(&self, seed: i64, chunk_x: i32, chunk_z: i32) -> ChunkBiomes {
        let origin = [chunk_x * 2 - 1, LATTICE_MIN_Y, chunk_z * 2 - 1];
        let mut lattice = Vec::with_capacity(LATTICE_COLUMNS * LATTICE_COLUMNS * LATTICE_ROWS);
        for gx in 0..LATTICE_COLUMNS as i32 {
            for gz in 0..LATTICE_COLUMNS as i32 {
                let column = self.lattice_column(origin[0] + gx, origin[2] + gz);
                for (row, biome) in column.into_iter().enumerate() {
                    let at = [origin[0] + gx, origin[1] + row as i32, origin[2] + gz];
                    lattice.push(LatticePoint { biome, jitter: jitter(seed, at) });
                }
            }
        }
        let mut ids = vec![0; 16 * 16 * HEIGHT];
        let mut picks = vec![[0; 3]; 16 * 16 * HEIGHT];
        for y in MIN_Y..MIN_Y + HEIGHT as i32 {
            for z in 0..16 {
                for x in 0..16 {
                    let block = [chunk_x * 16 + x as i32, y, chunk_z * 16 + z as i32];
                    let cell = [block[0] >> 3, block[1] >> 3, block[2] >> 3];
                    let base = (((cell[0] - origin[0]) as usize * LATTICE_COLUMNS) + (cell[2] - origin[2]) as usize) * LATTICE_ROWS
                        + (cell[1] - origin[1]) as usize;
                    let frac = block.map(|v| (v & 7) as f32 * 0.125);
                    let picked = pick(&lattice, base, frac);
                    ids[block_index(x, y, z)] = lattice[picked].biome;
                    let row = picked % LATTICE_ROWS;
                    let column = picked / LATTICE_ROWS;
                    let (gx, gz) = (column / LATTICE_COLUMNS, column % LATTICE_COLUMNS);
                    picks[block_index(x, y, z)] = [origin[0] + gx as i32, origin[1] + row as i32, origin[2] + gz as i32];
                }
            }
        }
        ChunkBiomes { ids, picks }
    }
}

/// The lattice corner nearest the block after jitter; ties keep the earlier corner, in BDS's
/// order z, y, zy, x, xz, xy, xyz (142f67a10).
fn pick(lattice: &[LatticePoint], base: usize, [fx, fy, fz]: [f32; 3]) -> usize {
    let (y_step, z_step, x_step) = (1, LATTICE_ROWS, LATTICE_ROWS * LATTICE_COLUMNS);
    let corners = [
        (0, [fx, fy, fz]),
        (z_step, [fx, fy, fz - 1.0]),
        (y_step, [fx, fy - 1.0, fz]),
        (z_step + y_step, [fx, fy - 1.0, fz - 1.0]),
        (x_step, [fx - 1.0, fy, fz]),
        (x_step + z_step, [fx - 1.0, fy, fz - 1.0]),
        (x_step + y_step, [fx - 1.0, fy - 1.0, fz]),
        (x_step + z_step + y_step, [fx - 1.0, fy - 1.0, fz - 1.0]),
    ];
    let mut best = (base, f32::MAX);
    for (offset, [dx, dy, dz]) in corners {
        let [jx, jy, jz] = lattice[base + offset].jitter;
        let (ex, ey, ez) = (jx + dx, jy + dy, jz + dz);
        let distance = ez * ez + ey * ey + ex * ex;
        if distance < best.1 {
            best = (base + offset, distance);
        }
    }
    best.0
}
