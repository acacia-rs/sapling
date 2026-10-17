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
        let grid = |octaves: &[ImprovedNoise], scales: [f32; 2]| -> Box<CellGrid> {
            fill_grid::<CELL_ROWS>(octaves, origin, scales).into_boxed_slice().try_into().expect("a full grid")
        };
        [grid(&self.main, [MAIN_XZ_SCALE, MAIN_Y_SCALE]), grid(&self.min_limit, [XZ_SCALE, XZ_SCALE]), grid(&self.max_limit, [XZ_SCALE, XZ_SCALE])]
    }
}

/// `count` octaves of one `PerlinNoise` as `LegacyBlendedNoise::addOctave` (145e29ee0) builds them off a
/// sequential random, highest frequency first.
pub fn legacy_octaves(random: &mut impl NoiseRandom, count: usize) -> Vec<ImprovedNoise> {
    (0..count).map(|_| ImprovedNoise::new_blended(random)).collect()
}

const COLUMNS: usize = CELL_COLUMNS * CELL_COLUMNS;

/// The grid by row, a row's columns side by side (column `gx * 5 + gz`) so an octave runs them in lanes.
type Rows<const ROWS: usize> = [[f32; COLUMNS]; ROWS];

/// The octaves summed over a 5x5 cell grid `ROWS` tall (`LegacyBlendedNoise::fillOctave` per octave), laid
/// out `[x][z][y]`; octave k samples at `scales * 2^-k` and is divided by `2^-k`.
pub fn fill_grid<const ROWS: usize>(octaves: &[ImprovedNoise], origin: [f32; 3], [xz_scale, y_scale]: [f32; 2]) -> Vec<f32> {
    let mut rows: Box<Rows<ROWS>> = Box::new([[0.0; COLUMNS]; ROWS]);
    let mut factor = 1.0f32;
    for octave in octaves {
        let (xz, y) = (xz_scale * factor, y_scale * factor);
        fill_octave(&Octave::<ROWS>::new(octave, origin, [xz, y, xz], factor), &mut rows);
        factor *= 0.5;
    }
    let mut out = vec![0.0f32; COLUMNS * ROWS];
    for (gy, row) in rows.iter().enumerate() {
        for (column, &value) in row.iter().enumerate() {
            out[column * ROWS + gy] = value;
        }
    }
    out
}

/// One octave over the grid (145e23fc0, general branch): each row's y lattice index and fraction,
/// each column's x and z.
struct Octave<'a, const ROWS: usize> {
    noise: &'a ImprovedNoise,
    y_scale: f32,
    inverse_factor: f32,
    fudge_index: i32,
    fudge_frac: f32,
    y: [f32; ROWS],
    ix: [i32; COLUMNS],
    iz: [i32; COLUMNS],
    fx: [f32; COLUMNS],
    fz: [f32; COLUMNS],
    sx: [f32; COLUMNS],
    sz: [f32; COLUMNS],
}

impl<'a, const ROWS: usize> Octave<'a, ROWS> {
    fn new(noise: &'a ImprovedNoise, origin: [f32; 3], scale: [f32; 3], factor: f32) -> Self {
        let fudge_y = scale[1] * 0.0 + noise.yo;
        let fudge_index = fudge_y as i32;
        let mut octave = Octave {
            noise,
            y_scale: scale[1],
            inverse_factor: 1.0 / factor,
            fudge_index,
            fudge_frac: fudge_y - fudge_index as f32,
            y: std::array::from_fn(|gy| ((gy as f32) + origin[1]) * scale[1] + noise.yo),
            ix: [0; COLUMNS],
            iz: [0; COLUMNS],
            fx: [0.0; COLUMNS],
            fz: [0.0; COLUMNS],
            sx: [0.0; COLUMNS],
            sz: [0.0; COLUMNS],
        };
        for c in 0..COLUMNS {
            let (gx, gz) = (c / CELL_COLUMNS, c % CELL_COLUMNS);
            (octave.ix[c], octave.fx[c]) = split(((gx as f32) + origin[0]) * scale[0] + noise.xo);
            (octave.iz[c], octave.fz[c]) = split(((gz as f32) + origin[2]) * scale[2] + noise.zo);
            octave.sx[c] = smoothstep(octave.fx[c]);
            octave.sz[c] = smoothstep(octave.fz[c]);
        }
        octave
    }

    /// Row `gy`'s lattice index and fraction, and the y fraction its gradients take when the last
    /// computed row had lattice index `last_iy` (None: they are reused).
    fn row(&self, gy: usize, last_iy: i32) -> (i32, f32, Option<f32>) {
        let y = self.y[gy];
        let floor = super::floor(y);
        let iy = floor as i32 & 0xFF;
        let fy = y - floor;
        let gradient_y = if y as i32 == self.fudge_index {
            Some(self.fudge_frac)
        } else if iy == last_iy {
            None
        } else {
            Some(fy - super::floor(fy / self.y_scale) * self.y_scale)
        };
        (iy, fy, gradient_y)
    }

    fn corners(&self, c: usize, iy: i32, gy_frac: f32) -> [f32; 4] {
        x_lerped_corners(self.noise, [self.ix[c], iy, self.iz[c]], [self.fx[c], gy_frac, self.fz[c]], self.sx[c])
    }

    fn sample(&self, c: usize, corners: [f32; 4], sy: f32) -> f32 {
        let near = (corners[1] - corners[0]) * sy + corners[0];
        let far = (corners[3] - corners[2]) * sy + corners[2];
        ((far - near) * self.sz[c] + near) * self.inverse_factor
    }
}

/// Gradient sums are reused while the y lattice index matches the last computed sample, even across
/// columns, as BDS does. Rows decide reuse alike in every column (the last row of one column leaves
/// its index for the next), so the columns run together row by row, unless a column's first row
/// would reuse the previous column's last gradients: then they run in BDS's order, one by one.
fn fill_octave<const ROWS: usize>(octave: &Octave<ROWS>, rows: &mut Rows<ROWS>) {
    let last = octave.row(ROWS - 1, -1).0;
    if octave.row(0, last).2.is_none() {
        return fill_octave_serial(octave, rows);
    }
    let mut corners = [[0.0f32; COLUMNS]; 4];
    let mut last_iy = -1;
    for (gy, row) in rows.iter_mut().enumerate() {
        let (iy, fy, gradient_y) = octave.row(gy, last_iy);
        if let Some(gy_frac) = gradient_y {
            for gx in 0..CELL_COLUMNS {
                let hashes = xy_hashes(octave.noise, octave.ix[gx * CELL_COLUMNS], iy);
                for c in gx * CELL_COLUMNS..(gx + 1) * CELL_COLUMNS {
                    let computed = lerped_from(octave.noise, hashes, octave.iz[c], [octave.fx[c], gy_frac, octave.fz[c]], octave.sx[c]);
                    (0..4).for_each(|i| corners[i][c] = computed[i]);
                }
            }
            last_iy = iy;
        }
        let sy = smoothstep(fy);
        for (c, out) in row.iter_mut().enumerate() {
            *out += octave.sample(c, [corners[0][c], corners[1][c], corners[2][c], corners[3][c]], sy);
        }
    }
}

fn fill_octave_serial<const ROWS: usize>(octave: &Octave<ROWS>, rows: &mut Rows<ROWS>) {
    let mut last_iy = -1i32;
    let mut corners = [0.0f32; 4];
    for c in 0..COLUMNS {
        for (gy, row) in rows.iter_mut().enumerate() {
            let (iy, fy, gradient_y) = octave.row(gy, last_iy);
            if let Some(gy_frac) = gradient_y {
                corners = octave.corners(c, iy, gy_frac);
                last_iy = iy;
            }
            row[c] += octave.sample(c, corners, smoothstep(fy));
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
    lerped_from(noise, xy_hashes(noise, ix, iy), iz, frac, sx)
}

/// The permutation chain's x and y steps (aa, ab, ba, bb), shared by the columns of one x.
fn xy_hashes(noise: &ImprovedNoise, ix: i32, iy: i32) -> [i32; 4] {
    let a = noise.hash_at(ix) + iy;
    let b = noise.hash_at(ix + 1) + iy;
    [noise.hash_at(a), noise.hash_at(a + 1), noise.hash_at(b), noise.hash_at(b + 1)]
}

fn lerped_from(noise: &ImprovedNoise, [aa, ab, ba, bb]: [i32; 4], iz: i32, [fx, fy, fz]: [f32; 3], sx: f32) -> [f32; 4] {
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
