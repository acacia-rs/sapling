//! Noise caves on a density cell corner (BDS Cavifier 1481e10d0): Java 1.18.1's spaghetti,
//! cheese, entrances and pillars, with BDS's operation order and its x128 density scale.

use super::noises::CaveNoises;
use crate::shaper::DENSITY_SCALE;

/// Below this the corner is left alone (Java's -64 bail-out, scaled differently).
const SKIP_BELOW: f32 = -150.0;
/// Java 1.5625 times 128: the sloped-cheese threshold between entrances and caverns.
const CAVERN_THRESHOLD: f32 = 200.0;
/// `minY * 0.125`, the spaghetti-2d elevation floor.
const MIN_CELL_Y: f32 = -8.0;

/// `(value + 1) * 0.5 * (max - min) + min`, BDS's `sampleNoiseAndMapToRange`.
fn map_unit(value: f32, scale: f32, min: f32) -> f32 {
    (value + 1.0) * 0.5 * scale + min
}

fn clamp_unit(value: f32) -> f32 {
    if 1.0 < value { 1.0 } else { value.max(-1.0) }
}

impl CaveNoises {
    pub fn carve(&self, [x, y, z]: [f32; 3], density: f32) -> f32 {
        if density < SKIP_BELOW {
            return density;
        }
        let modulator = map_unit(self.spaghetti_roughness_modulator.sample(x, y, z), 0.1, 0.0);
        let roughness = modulator * (0.4 - self.spaghetti_roughness.sample(x, y, z).abs());

        let rarity = quantize_3d(self.spaghetti_3d_rarity.sample(x + x, y, z + z));
        let thickness = map_unit(self.spaghetti_3d_thickness.sample(x, y, z), 0.023, 0.065);
        let inverse = 1.0 / rarity;
        let (sx, sy, sz) = (inverse * x, y * inverse, z * inverse);
        let first = (self.spaghetti_3d_1.sample(sx, sy, sz) * rarity).abs() - thickness;
        let second = (self.spaghetti_3d_2.sample(sx, sy, sz) * rarity).abs() - thickness;
        let spaghetti_3d = clamp_unit(second.max(first));

        let entrance_t = (y + 10.0) / 40.0;
        let entrance_slope = if 0.0 > entrance_t {
            0.3
        } else if entrance_t > 1.0 {
            0.0
        } else {
            0.3 - entrance_t * 0.3
        };
        let entrance = self.cave_entrance.sample(0.75 * x, y * 0.5, z * 0.75) + 0.37 + entrance_slope;
        let spaghetti = (spaghetti_3d + roughness).min(entrance);

        if CAVERN_THRESHOLD > density {
            return (spaghetti * DENSITY_SCALE * 5.0).min(density);
        }

        let cheese = clamp_unit(self.cave_cheese.sample(x, y * 0.666_666_7, z) + 0.27);
        let sloped = (density - 200.0) * 1.28;
        let cheese_slope = if 0.0 > sloped {
            0.5
        } else if sloped > 1.0 {
            0.0
        } else {
            0.5 - sloped * 0.5
        };
        let layer = self.cave_layer.sample(x, y * 8.0, z);
        let caverns = cheese + cheese_slope + layer * layer * 4.0;

        let rarity_2d = quantize_2d(self.spaghetti_2d_modulator.sample(x + x, y, z + z));
        let thickness_2d = self.spaghetti_2d_thickness.sample(x + x, y, z + z);
        let inverse_2d = 1.0 / rarity_2d;
        let noise_2d = self.spaghetti_2d.sample(inverse_2d * x, y * inverse_2d, z * inverse_2d);
        let elevation_noise = self.spaghetti_2d_elevation.sample(x, 0.0, z);
        let mut elevation = MIN_CELL_Y;
        if (elevation + -8.0).abs() > 1.192_092_9e-7 {
            elevation += (8.0 - elevation) * ((elevation_noise + 1.0) * 0.5);
        }
        let thickness_2d = map_unit(thickness_2d, 0.699_999_9, 0.6);
        let ridge = -0.083 * thickness_2d + (rarity_2d * noise_2d).abs();
        let height = (y * -0.125 + elevation).abs() - thickness_2d;
        let spaghetti_2d = clamp_unit((height * height * height).max(ridge));

        let carved = caverns.min(spaghetti.min(roughness + spaghetti_2d));

        let rareness = (self.pillar_rareness.sample(x, y, z) + 1.0) * 0.5;
        let rareness = rareness + rareness + 0.0;
        // BDS calls the CRT's `powf(t, 3)`, rounded correctly but for 1 in 16k; libm's differs per platform.
        let t = map_unit(self.pillar_thickness.sample(x, y, z), 1.1, 0.0) as f64;
        let pillar_thickness = (t * t * t) as f32;
        let pillar = self.pillar.sample(x * 25.0, y * 0.3, z * 25.0);
        let pillar = ((pillar + pillar) - rareness) * pillar_thickness;
        let pillar = if 0.03 < pillar { pillar } else { -f32::MAX };
        carved.max(pillar) * DENSITY_SCALE
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
