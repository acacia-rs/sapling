//! The density at every cell corner of a chunk (14822d4b0): blended base noise plus the shaped
//! height gradient, noise caves, top and bottom slides; and the noodle-cave noises beside it.

use super::Density;
use crate::climate::QuartClimate;
use crate::noise::{CELL_COLUMNS, CELL_COUNT, CELL_ROWS, CellGrid, LANES, Lanes};

pub const MIN_CELL_Y: i32 = -8;
/// `maxY - minY` in cells, for the top slide (the dimension is 384 tall).
const CELL_COUNT_Y: i32 = 48;

pub struct Cells {
    pub density: Box<CellGrid>,
    /// Noodle toggle, thickness, ridge A, ridge B; 1.0 where the corner density is below -150
    /// (BDS fills its interpolator buffers with 1.0, 14822ed79 and 1481f0380).
    pub noodle: [Box<CellGrid>; 4],
    /// Ore veininess, vein A, vein B, clamped to [-1, 1]; 1.0 below -150 as the noodles.
    pub veins: [Box<CellGrid>; 3],
}

pub fn cell_index(x: usize, z: usize, y: usize) -> usize {
    (x * CELL_COLUMNS + z) * CELL_ROWS + y
}

impl Density {
    pub fn cells(&self, chunk_x: i32, chunk_z: i32, climate: &impl Fn(i32, i32) -> QuartClimate) -> Cells {
        let origin = [(chunk_x * 4) as f32, MIN_CELL_Y as f32, (chunk_z * 4) as f32];
        let blended = self.blended.fill(origin);
        let mut density = Box::new([0.0f32; CELL_COUNT]);
        let mut noodle = std::array::from_fn(|_| Box::new([1.0f32; CELL_COUNT]));
        let mut veins = std::array::from_fn(|_| Box::new([1.0f32; CELL_COUNT]));
        let ys: [f32; CELL_ROWS] = std::array::from_fn(|cy| (origin[1] + cy as f32) * 8.0);
        for cx in 0..CELL_COLUMNS {
            let x = ((cx as f32) + origin[0]) * 4.0;
            for cz in 0..CELL_COLUMNS {
                let z = ((cz as f32) + origin[2]) * 4.0;
                let column = climate(chunk_x * 4 + cx as i32, chunk_z * 4 + cz as i32);
                let mut carved: [f32; CELL_ROWS] = std::array::from_fn(|cy| self.uncarved_density(blended[cell_index(cx, cz, cy)], &column, cy as i32));
                self.caves.carve_column(x, z, &ys, &mut carved);
                let mut kept = [0usize; CELL_ROWS];
                let mut count = 0;
                for cy in 0..CELL_ROWS {
                    let i = cell_index(cx, cz, cy);
                    let value = bottom_slide(top_slide(carved[cy], CELL_COUNT_Y - 8 - cy as i32), cy as i32);
                    density[i] = value;
                    if -150.0 <= value {
                        kept[count] = cy;
                        count += 1;
                    }
                }
                // The column's kept corners in lanes; a short last group repeats its last corner.
                for group in kept[..count].chunks(LANES) {
                    let ys: Lanes = std::array::from_fn(|l| ys[group[l.min(group.len() - 1)]]);
                    let lanes = |v: f32| [v; LANES];
                    let times = |v: &Lanes, k: f32| -> Lanes { v.map(|v| v * k) };
                    let (x15, x4, x83) = (lanes(x * 1.5), lanes(x * 4.0), lanes(x * 2.666_666_7));
                    let (z15, z4, z83) = (lanes(z * 1.5), lanes(z * 4.0), lanes(z * 2.666_666_7));
                    let samples = [
                        self.veins.veininess.sample_lanes(&x15, &times(&ys, 1.5), &z15),
                        self.veins.vein_a.sample_lanes(&x4, &times(&ys, 4.0), &z4),
                        self.veins.vein_b.sample_lanes(&x4, &times(&ys, 4.0), &z4),
                        self.noodles.toggle.sample_lanes(&lanes(x), &ys, &lanes(z)),
                        self.noodles.thickness.sample_lanes(&lanes(x), &ys, &lanes(z)),
                        self.noodles.ridge_a.sample_lanes(&x83, &times(&ys, 2.666_666_7), &z83),
                        self.noodles.ridge_b.sample_lanes(&x83, &times(&ys, 2.666_666_7), &z83),
                    ];
                    for (l, &cy) in group.iter().enumerate() {
                        let i = cell_index(cx, cz, cy);
                        (0..3).for_each(|k| veins[k][i] = clamp_unit(samples[k][l]));
                        (0..4).for_each(|k| noodle[k][i] = clamp_unit(samples[3 + k][l]));
                    }
                }
            }
        }
        Cells { density, noodle, veins }
    }

    #[doc(hidden)]
    pub fn blended_offsets(&self) -> [Vec<[f32; 3]>; 3] {
        self.blended.offsets()
    }

    /// The blended noise's three octave sums over a chunk's corners (examples/density_dump_diff.rs).
    #[doc(hidden)]
    pub fn blended_parts(&self, chunk_x: i32, chunk_z: i32) -> [Box<CellGrid>; 3] {
        self.blended.parts([(chunk_x * 4) as f32, MIN_CELL_Y as f32, (chunk_z * 4) as f32])
    }

    /// A corner's density before the noise caves and the slides.
    fn uncarved_density(&self, blended: f32, column: &QuartClimate, cy: i32) -> f32 {
        let block_y = ((MIN_CELL_Y + cy) * 8) as f32;
        let height = (block_y * -0.007_812_5 + 1.0 + column.offset + column.jagged) * column.factor;
        let height = height * if height > 0.0 { 4.0 } else { 1.0 };
        height + blended
    }
}

/// Toward -10 (Java -0.078125) over the top two cells.
pub fn top_slide(density: f32, cells_below_top: i32) -> f32 {
    let t = cells_below_top as f32 * 0.5;
    if 0.0 > t {
        -10.0
    } else if t > 1.0 {
        density
    } else {
        (density + 10.0) * t + -10.0
    }
}

/// Toward 15 (Java 0.1171875) over the bottom three cells.
pub fn bottom_slide(density: f32, cy: i32) -> f32 {
    let t = cy as f32 / 3.0;
    if 1.0 < t { density } else { (density + -15.0) * t + 15.0 }
}

fn clamp_unit(value: f32) -> f32 {
    if 1.0 < value { 1.0 } else { value.max(-1.0) }
}
