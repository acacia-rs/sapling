//! Java `BlendedNoise` as BDS's `LegacyBlendedNoise`: whole-chunk octave fills over the density
//! cell grid (14822d4b0 calling 145e23fc0), including BDS's y-fudge shortcut and its gradient
//! reuse between samples.

use super::NoiseRandom;
use super::improved::{ImprovedNoise, smoothstep};

/// The density cell grid of one chunk: 5x5 corners, 41 tall, laid out `[x][z][y]`.
pub const CELL_COLUMNS: usize = 5;
pub const CELL_ROWS: usize = 41;
pub const CELL_COUNT: usize = CELL_COLUMNS * CELL_COLUMNS * CELL_ROWS;

pub type CellGrid = [f32; CELL_COUNT];

const XZ_SCALE: f32 = 684.412;
const MAIN_XZ_SCALE: f32 = 8.555_15;
const MAIN_Y_SCALE: f32 = 4.277_575;

#[derive(Debug, Clone)]
pub struct BlendedNoise {
    min_limit: Vec<ImprovedNoise>,
    max_limit: Vec<ImprovedNoise>,
    main: Vec<ImprovedNoise>,
}

impl BlendedNoise {
    /// Octaves come off one sequential random, highest frequency first (145e24940).
    pub fn new(random: &mut impl NoiseRandom) -> Self {
        let mut octaves = |count: usize| (0..count).map(|_| ImprovedNoise::new_blended(random)).collect::<Vec<_>>();
        let min_limit = octaves(16);
        let max_limit = octaves(16);
        let main = octaves(8);
        Self { min_limit, max_limit, main }
    }

    /// The blended value at every cell corner, unscaled (Java's value times 128). `origin` is
    /// the grid's first corner in cell units.
    pub fn fill(&self, origin: [f32; 3]) -> Box<CellGrid> {
        let [main, min, max] = self.parts(origin);
        let mut out = Box::new([0.0f32; CELL_COUNT]);
        for i in 0..CELL_COUNT {
            let low = min[i] * 0.001_953_125;
            let t = (main[i] / 10.0 + 1.0) * 0.5;
            out[i] = if t < 0.0 {
                low
            } else if t > 1.0 {
                max[i] * 0.001_953_125
            } else {
                low + (max[i] * 0.001_953_125 - low) * t
            };
        }
        out
    }

    /// Every octave's offsets, main then lower then upper (examples/octave_offsets.rs).
    #[doc(hidden)]
    pub fn offsets(&self) -> [Vec<[f32; 3]>; 3] {
        [&self.main, &self.min_limit, &self.max_limit].map(|octaves| octaves.iter().map(|o| [o.xo, o.yo, o.zo]).collect())
    }

    /// The main, lower and upper octave sums (BDS's three stack buffers; examples/density_dump_diff.rs).
    #[doc(hidden)]
    pub fn parts(&self, origin: [f32; 3]) -> [Box<CellGrid>; 3] {
        [
            fill_octaves(&self.main, origin, |f| [MAIN_XZ_SCALE * f, MAIN_Y_SCALE * f]),
            fill_octaves(&self.min_limit, origin, |f| [XZ_SCALE * f, XZ_SCALE * f]),
            fill_octaves(&self.max_limit, origin, |f| [XZ_SCALE * f, XZ_SCALE * f]),
        ]
    }
}

fn fill_octaves(octaves: &[ImprovedNoise], origin: [f32; 3], scales: impl Fn(f32) -> [f32; 2]) -> Box<CellGrid> {
    let mut out = Box::new([0.0f32; CELL_COUNT]);
    let mut factor = 1.0f32;
    for octave in octaves {
        let [xz, y] = scales(factor);
        fill_octave(octave, &mut out, origin, [xz, y, xz], factor);
        factor *= 0.5;
    }
    out
}

/// One octave added into the grid (145e23fc0, general branch). Gradient sums are reused while
/// the y lattice index matches the last computed sample, even across columns, as BDS does.
fn fill_octave(noise: &ImprovedNoise, out: &mut CellGrid, origin: [f32; 3], scale: [f32; 3], factor: f32) {
    let inverse_factor = 1.0 / factor;
    let fudge_y = scale[1] * 0.0 + noise.yo;
    let fudge_index = fudge_y as i32;
    let fudge_frac = fudge_y - fudge_index as f32;

    let mut last_iy = -1i32;
    let mut corners = [0.0f32; 4];
    let mut index = 0;
    for gx in 0..CELL_COLUMNS {
        let (ix, fx) = split(((gx as f32) + origin[0]) * scale[0] + noise.xo);
        let sx = smoothstep(fx);
        for gz in 0..CELL_COLUMNS {
            let (iz, fz) = split(((gz as f32) + origin[2]) * scale[2] + noise.zo);
            let sz = smoothstep(fz);
            for gy in 0..CELL_ROWS {
                let y = ((gy as f32) + origin[1]) * scale[1] + noise.yo;
                let floor = super::floor(y);
                let iy = floor as i32 & 0xFF;
                let fy = y - floor;
                let gradient_y = if y as i32 == fudge_index {
                    Some(fudge_frac)
                } else if iy == last_iy {
                    None
                } else {
                    Some(fy - super::floor(fy / scale[1]) * scale[1])
                };
                if let Some(gy_frac) = gradient_y {
                    corners = x_lerped_corners(noise, [ix, iy, iz], [fx, gy_frac, fz], sx);
                    last_iy = iy;
                }
                let sy = smoothstep(fy);
                let near = (corners[1] - corners[0]) * sy + corners[0];
                let far = (corners[3] - corners[2]) * sy + corners[2];
                out[index] += ((far - near) * sz + near) * inverse_factor;
                index += 1;
            }
        }
    }
}

fn split(value: f32) -> (i32, f32) {
    let floor = super::floor(value);
    (floor as i32 & 0xFF, value - floor)
}

/// The four x-interpolated gradient sums: (y0,z0), (y1,z0), (y0,z1), (y1,z1).
fn x_lerped_corners(noise: &ImprovedNoise, lattice: [i32; 3], frac: [f32; 3], sx: f32) -> [f32; 4] {
    let [ix, iy, iz] = lattice;
    let [fx, fy, fz] = frac;
    let a = noise.hash_at(ix) + iy;
    let b = noise.hash_at(ix + 1) + iy;
    let (aa, ab) = (noise.hash_at(a), noise.hash_at(a + 1));
    let (ba, bb) = (noise.hash_at(b), noise.hash_at(b + 1));
    let lerp_x = |h0: i32, h1: i32, y: f32, z: f32| {
        let d0 = super::improved::grad_dot(noise.hash_at(h0), fx, y, z);
        let d1 = super::improved::grad_dot(noise.hash_at(h1), fx - 1.0, y, z);
        (d1 - d0) * sx + d0
    };
    [
        lerp_x(aa + iz, ba + iz, fy, fz),
        lerp_x(ab + iz, bb + iz, fy - 1.0, fz),
        lerp_x(aa + iz + 1, ba + iz + 1, fy, fz - 1.0),
        lerp_x(ab + iz + 1, bb + iz + 1, fy - 1.0, fz - 1.0),
    ]
}
