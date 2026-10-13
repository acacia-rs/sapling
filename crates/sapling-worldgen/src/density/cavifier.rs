//! Noise caves on a density cell corner (BDS Cavifier 1481e10d0): Java 1.18.1's spaghetti,
//! cheese, entrances and pillars, with BDS's operation order and its x128 density scale. A cell
//! column's corners sample each noise in lanes (`NormalNoise::sample_lanes`, bit-identical); the
//! arithmetic on the samples is per corner as before.

use super::noises::CaveNoises;
use crate::noise::{CELL_ROWS, LANES, Lanes};
use crate::shaper::DENSITY_SCALE;

/// Below this the corner is left alone (Java's -64 bail-out, scaled differently).
const SKIP_BELOW: f32 = -150.0;
/// Java 1.5625 times 128: the sloped-cheese threshold between entrances and caverns.
const CAVERN_THRESHOLD: f32 = 200.0;
/// The 3D spaghetti's thickness range, 0.088 - 0.065 in f32: one step above 0.023 (14a922758).
const THICKNESS_3D_SPAN: f32 = 0.088 - 0.065;
/// `minY * 0.125`, the spaghetti-2d elevation floor.
const MIN_CELL_Y: f32 = -8.0;

/// `(value + 1) * 0.5 * (max - min) + min`, BDS's `sampleNoiseAndMapToRange`.
fn map_unit(value: f32, scale: f32, min: f32) -> f32 {
    (value + 1.0) * 0.5 * scale + min
}

fn clamp_unit(value: f32) -> f32 {
    if 1.0 < value { 1.0 } else { value.max(-1.0) }
}

/// The rows of `rows` in groups of `LANES`, a short last group repeating its last row, with their y.
fn groups<'a>(rows: &'a [usize], ys: &'a [f32]) -> impl Iterator<Item = (&'a [usize], Lanes)> + 'a {
    rows.chunks(LANES).map(move |group| (group, std::array::from_fn(|l| ys[group[l.min(group.len() - 1)]])))
}

impl CaveNoises {
    /// Carves a cell column at (x, z): `densities` (before carving, by row) become the carved ones.
    pub fn carve_column(&self, x: f32, z: f32, ys: &[f32; CELL_ROWS], densities: &mut [f32; CELL_ROWS]) {
        let kept: Vec<usize> = (0..CELL_ROWS).filter(|&row| !(densities[row] < SKIP_BELOW)).collect();
        let (mut spaghetti, mut roughness) = ([0.0f32; CELL_ROWS], [0.0f32; CELL_ROWS]);
        for (group, y) in groups(&kept, ys) {
            let (xs, zs) = ([x; LANES], [z; LANES]);
            let modulator = self.spaghetti_roughness_modulator.sample_lanes(&xs, &y, &zs);
            let rough = self.spaghetti_roughness.sample_lanes(&xs, &y, &zs);
            let rarity = self.spaghetti_3d_rarity.sample_lanes(&[x + x; LANES], &y, &[z + z; LANES]).map(quantize_3d);
            let thickness = self.spaghetti_3d_thickness.sample_lanes(&xs, &y, &zs);
            let inverse = rarity.map(|r| 1.0 / r);
            let (sx, sy, sz): (Lanes, Lanes, Lanes) = (inverse.map(|i| i * x), std::array::from_fn(|l| y[l] * inverse[l]), inverse.map(|i| z * i));
            let first = self.spaghetti_3d_1.sample_lanes(&sx, &sy, &sz);
            let second = self.spaghetti_3d_2.sample_lanes(&sx, &sy, &sz);
            let entrance = self.cave_entrance.sample_lanes(&[0.75 * x; LANES], &y.map(|y| y * 0.5), &[z * 0.75; LANES]);
            for (l, &row) in group.iter().enumerate() {
                let r = map_unit(modulator[l], 0.1, 0.0) * (0.4 - rough[l].abs());
                let t = map_unit(thickness[l], THICKNESS_3D_SPAN, 0.065);
                let spaghetti_3d = clamp_unit(((second[l] * rarity[l]).abs() - t).max((first[l] * rarity[l]).abs() - t));
                spaghetti[row] = (spaghetti_3d + r).min(entrance[l] + 0.37 + entrance_slope(y[l]));
                roughness[row] = r;
            }
        }
        let deep: Vec<usize> = kept.iter().copied().filter(|&row| !(CAVERN_THRESHOLD > densities[row])).collect();
        for &row in &kept {
            if CAVERN_THRESHOLD > densities[row] {
                densities[row] = (spaghetti[row] * DENSITY_SCALE * 5.0).min(densities[row]);
            }
        }
        let elevation_noise = if deep.is_empty() { 0.0 } else { self.spaghetti_2d_elevation.sample(x, 0.0, z) };
        for (group, y) in groups(&deep, ys) {
            let (xs, zs, x2, z2) = ([x; LANES], [z; LANES], [x + x; LANES], [z + z; LANES]);
            let cheese = self.cave_cheese.sample_lanes(&xs, &y.map(|y| y * 0.666_666_7), &zs);
            let layer = self.cave_layer.sample_lanes(&xs, &y.map(|y| y * 8.0), &zs);
            let rarity_2d = self.spaghetti_2d_modulator.sample_lanes(&x2, &y, &z2).map(quantize_2d);
            let thickness_2d = self.spaghetti_2d_thickness.sample_lanes(&x2, &y, &z2);
            let inverse = rarity_2d.map(|r| 1.0 / r);
            let (sx, sy, sz): (Lanes, Lanes, Lanes) = (inverse.map(|i| i * x), std::array::from_fn(|l| y[l] * inverse[l]), inverse.map(|i| z * i));
            let noise_2d = self.spaghetti_2d.sample_lanes(&sx, &sy, &sz);
            let rareness = self.pillar_rareness.sample_lanes(&xs, &y, &zs);
            let pillar_thickness = self.pillar_thickness.sample_lanes(&xs, &y, &zs);
            let pillar = self.pillar.sample_lanes(&[x * 25.0; LANES], &y.map(|y| y * 0.3), &[z * 25.0; LANES]);
            for (l, &row) in group.iter().enumerate() {
                let density = densities[row];
                let cheese = clamp_unit(cheese[l] + 0.27);
                let sloped = (density - 200.0) * 1.28;
                let cheese_slope = if 0.0 > sloped {
                    0.5
                } else if sloped > 1.0 {
                    0.0
                } else {
                    0.5 - sloped * 0.5
                };
                let caverns = cheese + cheese_slope + layer[l] * layer[l] * 4.0;

                let mut elevation = MIN_CELL_Y;
                if (elevation + -8.0).abs() > 1.192_092_9e-7 {
                    elevation += (8.0 - elevation) * ((elevation_noise + 1.0) * 0.5);
                }
                let thickness_2d = map_unit(thickness_2d[l], 0.699_999_9, 0.6);
                let ridge = -0.083 * thickness_2d + (rarity_2d[l] * noise_2d[l]).abs();
                let height = (y[l] * -0.125 + elevation).abs() - thickness_2d;
                let spaghetti_2d = clamp_unit((height * height * height).max(ridge));

                let carved = caverns.min(spaghetti[row].min(roughness[row] + spaghetti_2d));

                let rareness = (rareness[l] + 1.0) * 0.5;
                let rareness = rareness + rareness + 0.0;
                // BDS calls the CRT's `powf(t, 3)`, rounded correctly but for 1 in 16k; libm's differs per platform.
                let t = map_unit(pillar_thickness[l], 1.1, 0.0) as f64;
                let pillar_thickness = (t * t * t) as f32;
                let pillar = ((pillar[l] + pillar[l]) - rareness) * pillar_thickness;
                let pillar = if 0.03 < pillar { pillar } else { -f32::MAX };
                densities[row] = carved.max(pillar) * DENSITY_SCALE;
            }
        }
    }
}

fn entrance_slope(y: f32) -> f32 {
    let entrance_t = (y + 10.0) / 40.0;
    if 0.0 > entrance_t {
        0.3
    } else if entrance_t > 1.0 {
        0.0
    } else {
        0.3 - entrance_t * 0.3
    }
}

fn quantize_3d(value: f32) -> f32 {
    if -0.5 > value {
        0.75
    } else if 0.0 > value {
        1.0
    } else if 0.5 > value {
        1.5
    } else {
        2.0
    }
}

fn quantize_2d(value: f32) -> f32 {
    if -0.75 > value {
        0.5
    } else if -0.5 > value {
        0.75
    } else if 0.5 > value {
        1.0
    } else if 0.75 > value {
        2.0
    } else {
        3.0
    }
}
