//! A chunk column's shape, as `_prepareHeights` (14822eb60) fills it: cell densities
//! interpolated per block, noodle caves, the aquifer's barrier and fluid. Ore veins, the bedrock
//! floor, surfaces, carvers and features are not applied.

use crate::climate::QuartClimate;
use crate::density::{AquiferCache, Cells, Density, MIN_CELL_Y, cell_index};

pub const MIN_Y: i32 = -64;
pub const HEIGHT: usize = 384;
/// Generation stops at y 255 (BDS passes 0x140 as the height); above is air.
const GENERATED_CELLS: usize = 40;
const SEA_LEVEL_CELL: usize = ((crate::density::SEA_LEVEL - MIN_Y) >> 3) as usize;
const NOODLE_MIN_Y: i32 = MIN_Y + 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellKind {
    Air,
    Water,
    Lava,
    Solid,
}

/// One chunk column: `kind(x, y, z)` with y from -64 to 319.
#[derive(Clone)]
pub struct ChunkShape {
    kinds: Vec<CellKind>,
}

impl ChunkShape {
    fn index(x: usize, y: i32, z: usize) -> usize {
        ((y - MIN_Y) as usize * 16 + z) * 16 + x
    }

    pub fn kind(&self, x: usize, y: i32, z: usize) -> CellKind {
        self.kinds[Self::index(x, y, z)]
    }

    pub(crate) fn set(&mut self, x: usize, y: i32, z: usize, kind: CellKind) {
        self.kinds[Self::index(x, y, z)] = kind;
    }
}

/// The trilinear corner set of one cell, BDS's interpolator struct.
struct Corners([f32; 8]);

impl Corners {
    fn load(grid: &[f32], x: usize, z: usize, y: usize) -> Self {
        Self([
            grid[cell_index(x, z, y)],
            grid[cell_index(x + 1, z, y)],
            grid[cell_index(x, z + 1, y)],
            grid[cell_index(x + 1, z + 1, y)],
            grid[cell_index(x, z, y + 1)],
            grid[cell_index(x + 1, z, y + 1)],
            grid[cell_index(x, z + 1, y + 1)],
            grid[cell_index(x + 1, z + 1, y + 1)],
        ])
    }

    /// z first, then x, then y, each `(b - a) * t + a`, with t = 0.25*k and 0.125*k.
    fn at(&self, x: u8, y: u8, z: u8) -> f32 {
        let [x0z0y0, x1z0y0, x0z1y0, x1z1y0, x0z0y1, x1z0y1, x0z1y1, x1z1y1] = self.0;
        let tz = 0.25 * z as f32;
        let x0y0 = (x0z1y0 - x0z0y0) * tz + x0z0y0;
        let x1y0 = (x1z1y0 - x1z0y0) * tz + x1z0y0;
        let x0y1 = (x0z1y1 - x0z0y1) * tz + x0z0y1;
        let x1y1 = (x1z1y1 - x1z0y1) * tz + x1z0y1;
        let tx = 0.25 * x as f32;
        let y0 = (x1y0 - x0y0) * tx + x0y0;
        let y1 = (x1y1 - x0y1) * tx + x0y1;
        (y1 - y0) * (0.125 * y as f32) + y0
    }
}

impl Density {
    pub fn chunk_shape(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let climate = |qx: i32, qz: i32| -> QuartClimate { self.climate.sample(qx, qz) };
        let cells = self.cells(chunk_x, chunk_z, &climate);
        let mut aquifer = AquiferCache::default();
        let mut kinds = vec![CellKind::Air; 16 * 16 * HEIGHT];
        for cx in 0..4 {
            for cz in 0..4 {
                let rows = self.generated_rows(&cells, chunk_x * 4 + cx as i32, chunk_z * 4 + cz as i32, cx, cz);
                for cy in 0..rows {
                    let density = Corners::load(&cells.density[..], cx, cz, cy);
                    let noodle = cells.noodle.each_ref().map(|grid| Corners::load(&grid[..], cx, cz, cy));
                    for dz in 0..4u8 {
                        for dx in 0..4u8 {
                            for dy in 0..8u8 {
                                let (x, z) = (cx * 4 + dx as usize, cz * 4 + dz as usize);
                                let y = (MIN_CELL_Y + cy as i32) * 8 + dy as i32;
                                let position = [chunk_x * 16 + x as i32, y, chunk_z * 16 + z as i32];
                                let value = block_density(density.at(dx, dy, dz), &noodle, [dx, dy, dz], y);
                                kinds[ChunkShape::index(x, y, z)] = self.block_kind(&mut aquifer, position, value);
                            }
                        }
                    }
                }
            }
        }
        ChunkShape { kinds }
    }

    /// Cell rows to fill in one cell column: BDS skips rows whose four corner columns are all
    /// negative above both the preliminary surface and sea level.
    fn generated_rows(&self, cells: &Cells, qx: i32, qz: i32, cx: usize, cz: usize) -> usize {
        let surface = self.preliminary_surface(qx, qz);
        let surface_cell = (surface - MIN_Y).div_euclid(8);
        let mut rows = GENERATED_CELLS;
        if surface_cell < 39 {
            let limit = (surface_cell as usize).max(SEA_LEVEL_CELL);
            loop {
                let row = rows - 1;
                let corners = [(cx, cz), (cx + 1, cz), (cx, cz + 1), (cx + 1, cz + 1)];
                if corners.iter().any(|&(x, z)| cells.density[cell_index(x, z, row)] >= 0.0) {
                    break;
                }
                rows -= 1;
                if !(limit + 1 < rows) {
                    break;
                }
            }
        }
        rows
    }

    fn block_kind(&self, aquifer: &mut AquiferCache, position: [i32; 3], density: f32) -> CellKind {
        let mut density = density;
        if density > 0.0 {
            return CellKind::Solid;
        }
        let answer = self.aquifer(aquifer, position);
        density += answer.barrier;
        if density > 0.0 {
            CellKind::Solid
        } else if position[1] < answer.fluid.level {
            if answer.fluid.lava { CellKind::Lava } else { CellKind::Water }
        } else {
            CellKind::Air
        }
    }
}

/// Java's `noise/2 - noise^3/24` on the clamped, rescaled density, then noodle caves.
fn block_density(interpolated: f32, noodle: &[Corners; 4], [dx, dy, dz]: [u8; 3], y: i32) -> f32 {
    let v = interpolated * 0.005;
    let v = if 1.0 < v { 1.0 } else { v.max(-1.0) };
    let noise = v * v * v / -24.0 + v * 0.5;
    if y < NOODLE_MIN_Y || !(noise > 0.0) || 0.0 > noodle[0].at(dx, dy, dz) {
        return noise;
    }
    let t = (noodle[1].at(dx, dy, dz) + 1.0) * 0.5;
    let thickness = if 0.0 > t {
        0.05
    } else if t > 1.0 {
        0.1
    } else {
        t * 0.05 + 0.05
    };
    let ridge_a = (noodle[2].at(dx, dy, dz) * 1.5).abs() - thickness;
    let ridge_b = (noodle[3].at(dx, dy, dz) * 1.5).abs() - thickness;
    ridge_b.max(ridge_a).min(noise)
}
