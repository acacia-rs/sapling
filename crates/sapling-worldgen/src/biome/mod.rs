//! Per-block overworld biomes as BDS fills them (`BiomeSource3d::fillBiomes` 14805f140): the
//! multi-noise tree sampled on an 8-block lattice (14805eb90), then a jittered nearest-lattice-point
//! zoom to single blocks (148a72d90 offsets, 142f67a10 pick).

#[cfg(test)]
mod btree;
mod builder;
#[cfg(test)]
mod ids;
mod rtree;
#[cfg(test)]
mod tests;

pub use rtree::{ClimatePoint, Leaf, RTree};

use std::sync::OnceLock;

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

/// Bedrock biome ids (all below 256) per block, a byte each so neighbours' biomes stay cached cheaply.
pub struct ChunkBiomes {
    chunk: (i32, i32),
    ids: Vec<u8>,
    /// Which corner of its lattice cell each block took (`CORNERS`), for diagnosing.
    corners: Vec<u8>,
}

/// Lattice cell corners in BDS's tie order z, y, zy, x, xz, xy, xyz (142f67a10), as [x, y, z].
const CORNERS: [[i32; 3]; 8] = [[0, 0, 0], [0, 0, 1], [0, 1, 0], [0, 1, 1], [1, 0, 0], [1, 0, 1], [1, 1, 0], [1, 1, 1]];

fn block_index(x: usize, y: i32, z: usize) -> usize {
    ((y - MIN_Y) as usize * 16 + z) * 16 + x
}

impl ChunkBiomes {
    pub fn biome(&self, x: usize, y: i32, z: usize) -> u32 {
        u32::from(self.ids[block_index(x, y, z)])
    }

    /// The lattice point (x, y, z in lattice units) the block took its biome from; None inside a
    /// lattice cell of one biome, where no corner was picked.
    pub fn lattice_point(&self, x: usize, y: i32, z: usize) -> Option<[i32; 3]> {
        let corner = CORNERS.get(self.corners[block_index(x, y, z)] as usize)?;
        let block = [self.chunk.0 * 16 + x as i32, y, self.chunk.1 * 16 + z as i32];
        Some(std::array::from_fn(|axis| (block[axis] >> 3) + corner[axis]))
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

/// BDS's overworld biome tree, built once.
pub fn overworld_tree() -> &'static RTree {
    static TREE: OnceLock<RTree> = OnceLock::new();
    TREE.get_or_init(|| RTree::new(&builder::overworld_entries()))
}

impl ClimateNoises {
    /// One lattice column, bottom up, each search warm-started from the one below.
    fn lattice_column(&self, lx: i32, lz: i32) -> Vec<u32> {
        let climate = self.sample(lx * 2, lz * 2);
        let tree = overworld_tree();
        let mut warm = None;
        (0..LATTICE_ROWS as i32)
            .map(|row| {
                let y = (LATTICE_MIN_Y + row) * 8;
                let depth = y as f32 * -0.007_812_5 + 1.0 + climate.offset;
                let point = [climate.temperature, climate.humidity, climate.continentalness, climate.erosion, depth, climate.weirdness].map(quantize);
                let leaf = tree.nearest(&point, warm);
                warm = Some(leaf);
                tree.biome(leaf)
            })
            .collect()
    }

    /// The biome of one block without the lattice (`BiomeSource3d::_getBiome` 14805f850, climate
    /// 1481df3b0): the climate of its quart column, depth at the block's own y, no warm start.
    pub fn biome_at(&self, x: i32, y: i32, z: i32) -> u32 {
        let climate = self.sample(x >> 2, z >> 2);
        let depth = y as f32 * -0.007_812_5 + 1.0 + climate.offset;
        let point = [climate.temperature, climate.humidity, climate.continentalness, climate.erosion, depth, climate.weirdness].map(quantize);
        let tree = overworld_tree();
        tree.biome(tree.nearest(&point, None))
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
        let mut corners = vec![UNIFORM; 16 * 16 * HEIGHT];
        let point = |base: usize, [cx, cy, cz]: [i32; 3]| base + cx as usize * LATTICE_ROWS * LATTICE_COLUMNS + cz as usize * LATTICE_ROWS + cy as usize;
        for y in MIN_Y..MIN_Y + HEIGHT as i32 {
            for z in 0..16 {
                for x in 0..16 {
                    let block = [chunk_x * 16 + x as i32, y, chunk_z * 16 + z as i32];
                    let cell = [block[0] >> 3, block[1] >> 3, block[2] >> 3];
                    let base = (((cell[0] - origin[0]) as usize * LATTICE_COLUMNS) + (cell[2] - origin[2]) as usize) * LATTICE_ROWS
                        + (cell[1] - origin[1]) as usize;
                    let first = lattice[base].biome;
                    if CORNERS.iter().all(|&corner| lattice[point(base, corner)].biome == first) {
                        ids[block_index(x, y, z)] = first as u8;
                        continue;
                    }
                    let frac = block.map(|v| (v & 7) as f32 * 0.125);
                    let corner = pick(&lattice, base, frac);
                    ids[block_index(x, y, z)] = lattice[point(base, CORNERS[corner])].biome as u8;
                    corners[block_index(x, y, z)] = corner as u8;
                }
            }
        }
        ChunkBiomes { chunk: (chunk_x, chunk_z), ids, corners }
    }
}

/// `ChunkBiomes::corners` of a block whose lattice cell has one biome at every corner: any
/// corner gives it, so none was picked.
const UNIFORM: u8 = u8::MAX;

/// The index into `CORNERS` of the lattice corner nearest the block after jitter; ties keep the
/// earlier corner.
fn pick(lattice: &[LatticePoint], base: usize, frac: [f32; 3]) -> usize {
    let mut best = (0, f32::MAX);
    for (index, corner) in CORNERS.iter().enumerate() {
        let point = base + corner[0] as usize * LATTICE_ROWS * LATTICE_COLUMNS + corner[2] as usize * LATTICE_ROWS + corner[1] as usize;
        let [jx, jy, jz] = lattice[point].jitter;
        let [dx, dy, dz] = std::array::from_fn(|axis| frac[axis] - corner[axis] as f32);
        let (ex, ey, ez) = (jx + dx, jy + dy, jz + dz);
        let distance = ez * ez + ey * ey + ex * ex;
        if distance < best.1 {
            best = (index, distance);
        }
    }
    best.0
}
