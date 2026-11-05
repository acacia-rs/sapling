//! Finding the generator's structures and biomes from a place, as `/locate` does.

use crate::World;

/// BDS's `/locate biome` (143aad6a0, 143aaa5d0): a box this far round the sender searched on a lattice of
/// `COARSE`, then the hit's surface, else a `FINE` search under it and an open cell near that.
const REACH: i32 = 4000;
const COARSE: i32 = 64;
const FINE: i32 = 16;
/// Where the search starts again when the first finds nothing (143aad6a0).
const RETRY_SHIFT: i32 = 32;

impl World {
    /// The block column `/locate structure` answers for the structure `name` from block (x, z)
    /// ([`crate::WorldStorage::locate_structure`]); `None` without a generator or where none is found.
    pub fn locate_structure(&self, name: &str, x: i32, z: i32) -> Option<[i32; 2]> {
        self.loader.as_ref()?.storage().locate_structure(name, x, z)
    }

    /// The cell `/locate biome` answers for `biome` from `from`; `None` without a generator or where the
    /// search finds none.
    pub fn locate_biome(&self, biome: u32, from: [i32; 3]) -> Option<[i32; 3]> {
        let storage = self.loader.as_ref()?.storage();
        storage.generated_biome(from[0], from[1], from[2])?;
        let matches = |[x, y, z]: [i32; 3]| storage.generated_biome(x, y, z) == Some(biome);
        let (bottom, top) = (self.dim.min_y, self.dim.min_y + self.dim.height as i32);
        // The search's middle height under `top`, as BDS rounds it.
        let middle = |top: i32| (top - bottom - 4) / 2 + bottom + 5;
        let search = |[x, _, z]: [i32; 3]| {
            let first = lattice_search(&matches, [x, middle(top), z], [x - REACH, bottom + 5, z - REACH], [x + REACH, top, z + REACH], COARSE)?;
            let height = storage.generated_height(first[0], first[2])?;
            let surface = [first[0], height, first[2]];
            if matches(surface) {
                return Some(surface);
            }
            let (lo, hi) = ([first[0] - COARSE, bottom + 5, first[2] - COARSE], [first[0] + COARSE, height, first[2] + COARSE]);
            let second = lattice_search(&matches, [first[0], middle(height), first[2]], lo, hi, FINE)?;
            Some(self.open_cell_near(second, &matches))
        };
        search(from).or_else(|| search(from.map(|v| v + RETRY_SHIFT)))
    }

    /// 145808020: from 16 above `found` down in steps of two, the first cell within 16 across where the
    /// biome is wanted in it or the cell over it, both air in a loaded column; `found` when none is
    /// (bds-locate.txt: the deep dark answered on its lattice cell, unloaded, at 32 -43 -128).
    fn open_cell_near(&self, found: [i32; 3], matches: &dyn Fn([i32; 3]) -> bool) -> [i32; 3] {
        let air = |[x, y, z]: [i32; 3]| self.is_loaded(crate::ChunkPos::of_block(x, z)) && self.block(x, y, z) == self.dim.air;
        let low = (-FINE).max(self.dim.min_y + 1 - found[1]);
        let mut dy = FINE;
        while low < dy {
            for dx in -FINE..=FINE {
                for dz in -FINE..=FINE {
                    let at = [found[0] + dx, found[1] + dy, found[2] + dz];
                    let over = [at[0], at[1] + 1, at[2]];
                    if (matches(at) || matches(over)) && air(at) && air(over) {
                        return at;
                    }
                }
            }
            dy -= 2;
        }
        found
    }
}

/// 145807000: the lattice of `step` from `lo` covering `hi`, cut into blocks of 16 cells; blocks in shells
/// round the one holding `from`, each block's cells in shells round its cell nearest `from` (x, then y,
/// then z innermost), the first cell whose biome `matches`.
fn lattice_search(matches: &dyn Fn([i32; 3]) -> bool, from: [i32; 3], lo: [i32; 3], hi: [i32; 3], step: i32) -> Option<[i32; 3]> {
    let cells: [i32; 3] = std::array::from_fn(|a| ((hi[a] - lo[a]) as f32 * (1.0 / step as f32)).ceil() as i32);
    let hi: [i32; 3] = std::array::from_fn(|a| lo[a] + step * cells[a]);
    let start: [i32; 3] = std::array::from_fn(|a| ((from[a] - lo[a]) / step).clamp(0, cells[a]) >> 4);
    let blocks: [i32; 3] = std::array::from_fn(|a| (cells[a] as f32 * 0.0625).ceil() as i32);
    for ring in 0..*blocks.iter().max()? {
        for block in shell(start, ring) {
            if (0..3).any(|a| !(0..blocks[a]).contains(&block[a])) {
                continue;
            }
            let b_lo: [i32; 3] = std::array::from_fn(|a| (lo[a] + block[a] * 16 * step).clamp(lo[a], hi[a]));
            let b_hi: [i32; 3] = std::array::from_fn(|a| (lo[a] + (block[a] * 16 + 16) * step).clamp(lo[a], hi[a]));
            let dims: [i32; 3] = std::array::from_fn(|a| (b_hi[a] - b_lo[a]) / step);
            let near: [i32; 3] = std::array::from_fn(|a| (from[a].clamp(b_lo[a], b_hi[a]) - b_lo[a]) / step);
            for k in 0..=dims.iter().map(|d| d - 1).max()? {
                for cell in shell(near, k) {
                    if (0..3).any(|a| !(0..dims[a]).contains(&cell[a])) {
                        continue;
                    }
                    let at: [i32; 3] = std::array::from_fn(|a| b_lo[a] + cell[a] * step);
                    if matches(at) {
                        return Some(at);
                    }
                }
            }
        }
    }
    None
}

/// The cells at Chebyshev distance `ring` from `centre`, x outermost and z innermost.
fn shell(centre: [i32; 3], ring: i32) -> impl Iterator<Item = [i32; 3]> {
    let span = move |a: usize| centre[a] - ring..=centre[a] + ring;
    span(0)
        .flat_map(move |x| span(1).flat_map(move |y| span(2).map(move |z| [x, y, z])))
        .filter(move |cell| (0..3).any(|a| (cell[a] - centre[a]).abs() == ring))
}
