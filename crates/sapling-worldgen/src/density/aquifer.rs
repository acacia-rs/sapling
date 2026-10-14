//! Java 1.18.1 `NoiseBasedAquifer` as BDS has it (`Aquifer::Aquifer` 1481df940, substance
//! 1481dfe30, fluid status 1481e0af0). Fluid centers and statuses are pure functions of their
//! grid cell, cached per chunk as BDS's arrays are.

use super::Density;

pub const SEA_LEVEL: i32 = 63;
/// Below this BDS answers lava without sampling anything.
const LAVA_LEVEL: i32 = -54;
const WAY_BELOW_MIN_Y: i32 = -0x1000;

/// Java `SURFACE_SAMPLING_OFFSETS_IN_CHUNKS` (0x14a922f90).
const SURFACE_OFFSETS: [(i32, i32); 13] =
    [(-2, -1), (-1, -1), (0, -1), (1, -1), (-3, 0), (-2, 0), (-1, 0), (0, 0), (1, 0), (-2, 1), (-1, 1), (0, 1), (1, 1)];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluidStatus {
    pub level: i32,
    pub lava: bool,
}

/// What the aquifer decided for one block: the barrier to add to the density, the fluid to
/// place if the block stays empty, and whether that fluid gets a tick (BDS +0x25, Java's
/// `shouldScheduleFluidUpdate`): its two nearest centres are close to equidistant. The flag is
/// meaningful only below the fluid's level.
#[derive(Debug, Clone, Copy)]
pub struct AquiferAnswer {
    pub barrier: f32,
    pub fluid: FluidStatus,
    pub update: bool,
}

/// The grid cells a chunk's blocks reach: x and z from the chunk's minus one, y over the world's height.
const SPAN_XZ: i32 = 3;
const MIN_GY: i32 = -6;
const SPAN_Y: i32 = 34;

/// Each grid cell's centre and its fluid status once computed, densely over one chunk's cells (as
/// BDS's per-chunk arrays); cells beyond them are computed uncached.
pub struct AquiferCache {
    origin: (i32, i32, i32),
    cells: Vec<(Option<(i32, i32, i32)>, Option<FluidStatus>)>,
    /// The last block's grid cell and its twelve candidate centres with their slots: the blocks of a
    /// column ask the same cell up to twelve times running.
    last: Option<((i32, i32, i32), Candidates)>,
    /// For `last`, once asked: the status all twelve candidates share, if they do.
    shared: Option<Option<FluidStatus>>,
    /// For `last` and the block column last asked: each candidate's x and z terms of the distance.
    column: Option<((i32, i32), [i32; 12])>,
}

type Candidates = [((i32, i32, i32), Option<usize>); 12];

impl AquiferCache {
    pub fn new((chunk_x, chunk_z): (i32, i32)) -> Self {
        Self { origin: (chunk_x - 1, MIN_GY, chunk_z - 1), cells: vec![(None, None); (SPAN_XZ * SPAN_Y * SPAN_XZ) as usize], last: None, shared: None, column: None }
    }

    fn slot(&self, (gx, gy, gz): (i32, i32, i32)) -> Option<usize> {
        let (x, y, z) = (gx - self.origin.0, gy - self.origin.1, gz - self.origin.2);
        ((0..SPAN_XZ).contains(&x) && (0..SPAN_Y).contains(&y) && (0..SPAN_XZ).contains(&z)).then(|| ((x * SPAN_Y + y) * SPAN_XZ + z) as usize)
    }
}

impl Density {
    pub fn aquifer(&self, cache: &mut AquiferCache, [x, y, z]: [i32; 3]) -> AquiferAnswer {
        if y < LAVA_LEVEL {
            return AquiferAnswer { barrier: 0.0, fluid: FluidStatus { level: LAVA_LEVEL, lava: true }, update: y == LAVA_LEVEL - 1 };
        }
        let cell = ((x - 5).div_euclid(16), (y + 1).div_euclid(12), (z - 5).div_euclid(16));
        if !matches!(cache.last, Some((last, _)) if last == cell) {
            let candidates = self.candidates(cache, cell);
            cache.last = Some((cell, candidates));
            cache.shared = None;
            cache.column = None;
        }
        // With one status all around, every pressure is 0 and so is the barrier, whichever centres are
        // nearest; at or above its level the block is air, which needs no update flag.
        if y >= SEA_LEVEL {
            let shared = match cache.shared {
                Some(shared) => shared,
                None => {
                    let candidates = cache.last.expect("set above").1;
                    let first = self.fluid_status(cache, candidates[0]);
                    let shared = candidates.iter().all(|&c| self.fluid_status(cache, c) == first).then_some(first);
                    *cache.shared.insert(shared)
                }
            };
            if let Some(fluid) = shared.filter(|fluid| y >= fluid.level) {
                return AquiferAnswer { barrier: 0.0, fluid, update: false };
            }
        }
        // Borrowed, not copied: this runs for every open block.
        let AquiferCache { last, column, .. } = &mut *cache;
        let candidates = &last.as_ref().expect("set above").1;
        let flat = match *column {
            Some((at, flat)) if at == (x, z) => flat,
            _ => {
                let flat = candidates.map(|((cx, _, cz), _)| (cx - x) * (cx - x) + (cz - z) * (cz - z));
                *column = Some(((x, z), flat));
                flat
            }
        };
        let mut nearest = [(u32::MAX, 0); 3];
        for (i, ((_, cy, _), _)) in candidates.iter().enumerate() {
            let distance = (flat[i] + (cy - y) * (cy - y)) as u32;
            if nearest[0].0 >= distance {
                nearest = [(distance, i), nearest[0], nearest[1]];
            } else if nearest[1].0 >= distance {
                nearest = [nearest[0], (distance, i), nearest[1]];
            } else if nearest[2].0 >= distance {
                nearest[2] = (distance, i);
            }
        }
        let nearest = nearest.map(|(distance, i)| (distance, candidates[i]));
        let first = self.fluid_status(cache, nearest[0].1);
        let similarity_12 = similarity(nearest[0].0, nearest[1].0);
        let update = -0.4 < similarity_12;
        if !(-1.0 < similarity_12) {
            return AquiferAnswer { barrier: 0.0, fluid: first, update };
        }
        let second = self.fluid_status(cache, nearest[1].1);
        let third = self.fluid_status(cache, nearest[2].1);
        let mut barrier_noise = None;
        let mut pressure = |a: FluidStatus, b: FluidStatus| -> f32 {
            if y < a.level && y < b.level && a.lava != b.lava {
                return 1.0;
            }
            if a.level == b.level {
                return 0.0;
            }
            let center = (y as f32 + 0.5) - (a.level + b.level) as f32 * 0.5;
            let half_gap = (a.level - b.level).abs() as f32 * 0.5 - center.abs();
            let value = if center <= 0.0 {
                let v = half_gap + 3.0;
                v / if 0.0 < v { 3.0 } else { 10.0 }
            } else {
                let v = half_gap + 0.0;
                v / if 0.0 < v { 1.5 } else { 2.5 }
            };
            if value.abs() <= 2.0 {
                let noise = *barrier_noise.get_or_insert_with(|| self.aquifer_noises.barrier.sample(x as f32, y as f32 * 0.5, z as f32));
                value + noise
            } else {
                value
            }
        };
        let p13 = pressure(first, third);
        let p12 = pressure(first, second);
        let p23 = pressure(second, third);
        let similarity_13 = similarity(nearest[0].0, nearest[2].0).max(0.0);
        let similarity_23 = similarity(nearest[1].0, nearest[2].0).max(0.0);
        let others = (similarity_13 * p13).max(similarity_23 * p23);
        let barrier = (similarity_12.max(0.0) * 2.0 * p12.max(others)).max(0.0);
        AquiferAnswer { barrier, fluid: first, update }
    }

    /// The centres of the twelve grid cells around a block's, in the order they are compared.
    fn candidates(&self, cache: &mut AquiferCache, (gx, gy, gz): (i32, i32, i32)) -> Candidates {
        let mut candidates = [((0, 0, 0), None); 12];
        let mut i = 0;
        for dx in 0..=1 {
            for dy in -1..=1 {
                for dz in 0..=1 {
                    let cell = (gx + dx, gy + dy, gz + dz);
                    let slot = cache.slot(cell);
                    candidates[i] = (self.aquifer_center(cache, cell, slot), slot);
                    i += 1;
                }
            }
        }
        candidates
    }

    fn aquifer_center(&self, cache: &mut AquiferCache, cell: (i32, i32, i32), slot: Option<usize>) -> (i32, i32, i32) {
        let compute = || {
            let mut random = self.aquifer_noises.positional.at(cell.0, cell.1, cell.2);
            let dx = random.next_int_bounded(10);
            let dy = random.next_int_bounded(9);
            let dz = random.next_int_bounded(10);
            (cell.0 * 16 + dx, cell.1 * 12 + dy, cell.2 * 16 + dz)
        };
        match slot {
            Some(slot) => *cache.cells[slot].0.get_or_insert_with(compute),
            None => compute(),
        }
    }

    /// A centre and the slot of its grid cell, which holds its status.
    fn fluid_status(&self, cache: &mut AquiferCache, (center, slot): ((i32, i32, i32), Option<usize>)) -> FluidStatus {
        match slot {
            Some(slot) => *cache.cells[slot].1.get_or_insert_with(|| self.compute_fluid(center)),
            None => self.compute_fluid(center),
        }
    }

    fn compute_fluid(&self, (x, y, z): (i32, i32, i32)) -> FluidStatus {
        let global = FluidStatus { level: SEA_LEVEL, lava: false };
        let mut lowest_surface = 0x7fff;
        let mut near_surface = false;
        for (dx, dz) in SURFACE_OFFSETS {
            let surface = self.preliminary_surface((x + dx * 16) >> 2, (z + dz * 16) >> 2);
            let center = dx == 0 && dz == 0;
            if center && surface + 8 < y - 12 {
                return global;
            }
            let above = surface < y + 4;
            if (center || above) && surface + 8 < SEA_LEVEL {
                if above {
                    return global;
                }
                near_surface = true;
            }
            lowest_surface = lowest_surface.min(surface);
        }
        if self.is_deep_dark(x, y, z) {
            return FluidStatus { level: WAY_BELOW_MIN_Y, lava: false };
        }
        let mut weight = 1.0f32;
        if near_surface {
            let t = ((lowest_surface + 8) - y) as f32 * 0.015625;
            weight = if 0.0 > t { -0.0 } else if t <= 1.0 { -((1.0 - t) + -1.0) } else { 1.0 };
        }
        let floodedness = self.aquifer_noises.floodedness.sample(x as f32, y as f32 * 0.67, z as f32);
        let floodedness = if 1.0 < floodedness { 1.0 } else { floodedness.max(-1.0) };
        if weight * 1.1 + -0.3 < floodedness {
            return global;
        }
        if !(weight * 1.2 + -0.8 <= floodedness) {
            return FluidStatus { level: WAY_BELOW_MIN_Y, lava: false };
        }
        let (gx, gy, gz) = (x.div_euclid(16), y.div_euclid(40), z.div_euclid(16));
        let spread = self.aquifer_noises.spread.sample(gx as f32, gy as f32 / 1.4, gz as f32) * 10.0 / 3.0;
        let level = spread.floor() as i32 * 3 + gy * 40 + 20;
        let lava = level < -9 && {
            let lava = self.aquifer_noises.lava.sample(x.div_euclid(64) as f32, y.div_euclid(40) as f32, z.div_euclid(64) as f32);
            0.3 < lava.abs()
        };
        FluidStatus { level: level.min(lowest_surface), lava }
    }

    /// Java 1.19's deep-dark check: erosion below -0.225 and depth above 0.9.
    fn is_deep_dark(&self, x: i32, y: i32, z: i32) -> bool {
        let (shifted_x, shifted_z) = self.climate.shift(x >> 2, z >> 2);
        let point = self.climate.shaper_point(shifted_x, shifted_z);
        if -0.225 <= point.erosion {
            return false;
        }
        let depth = y as f32 * -0.007_812_5 + 1.0 + self.climate.shaper.raw_offset(&point) + crate::shaper::TerrainShaper::global_offset();
        !(depth <= 0.9)
    }
}

/// `1 - |a - b| / 25`, BDS's `|a-b| * -0.04 + 1`.
fn similarity(a: u32, b: u32) -> f32 {
    a.abs_diff(b) as f32 * -0.04 + 1.0
}
