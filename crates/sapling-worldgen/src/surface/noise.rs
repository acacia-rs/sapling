//! The noises the surface builders read: the depth noise ("minecraft:surface", 4 simplex octaves
//! summed by `loadChunk` into a 16x16 buffer), the material-adjustment noise (3 octaves from an MT
//! on the 32-bit seed, built in the generator's constructor 14821f6f0), and the builders' own.

use super::frozen::IcebergNoises;
use super::gradient::GradientNoises;
use super::mesa::MesaNoises;
use super::swamp::PuddleNoise;
use crate::noise::{SimplexNoise, legacy_simplex_octaves};
use crate::random::{Mt19937, PositionalFactory};

pub struct SurfaceNoises {
    depth: Vec<SimplexNoise>,
    adjustment: Vec<SimplexNoise>,
    pub mesa: MesaNoises,
    pub iceberg: IcebergNoises,
    pub puddle: PuddleNoise,
    pub gradients: GradientNoises,
}

impl SurfaceNoises {
    pub fn new(factory: &PositionalFactory, seed: i64) -> Self {
        let mut random = factory.from_hash_of("minecraft:surface");
        let depth = (0..4)
            .map(|_| {
                let offsets = [random.next_float(), random.next_float(), random.next_float()];
                SimplexNoise::new(offsets, |bound| random.next_int_bounded(bound))
            })
            .collect();
        let mut mt = Mt19937::new(seed as u32);
        mt.consume_count(20202);
        let adjustment = legacy_simplex_octaves(&mut mt, 3);
        Self {
            depth,
            adjustment,
            mesa: MesaNoises::new(seed),
            iceberg: IcebergNoises::new(seed),
            puddle: PuddleNoise::new(seed),
            gradients: GradientNoises::new(factory),
        }
    }

    /// The adjustment noise at `position * scale` (143907ca0): octaves at doubling wavelength,
    /// each divided by its scale, normalised by 1/7, with no origin offsets.
    pub fn adjustment(&self, [x, y, z]: [i32; 3], scale: f32) -> f32 {
        let mut total = 0.0f32;
        let mut octave_scale = 1.0f32;
        for octave in &self.adjustment {
            let [x, y, z] = [x, y, z].map(|v| v as f32 * scale * octave_scale);
            total = total + octave.value3(x, y, z) / octave_scale;
            octave_scale *= 0.5;
        }
        total * (1.0 / 7.0)
    }

    /// The depth noise at a block column (148211300, the loop before `buildChunkSurface`).
    pub fn depth(&self, x: i32, z: i32) -> f32 {
        let mut total = 0.0f32;
        let mut scale = 1.0f32;
        for octave in &self.depth {
            let [xo, zo] = octave.origin;
            let value = octave.value(x as f32 * scale * 0.125 + xo, z as f32 * scale * 0.125 + zo);
            total = value * (0.55 / scale) + total;
            scale *= 0.5;
        }
        total
    }
}

#[cfg(test)]
mod tests {
    /// `SAPLING_DEPTH_AT=x,z cargo test --lib depth_octaves -- --ignored --nocapture`: each octave's input and
    /// value at a column of seed 20261003, to set beside a debugger log of 148211af0.
    #[test]
    #[ignore]
    fn depth_octaves() {
        let at = std::env::var("SAPLING_DEPTH_AT").unwrap_or_else(|_| "1600014,1600103".into());
        let [x, z]: [i32; 2] = at.split(',').map(|v| v.parse().unwrap()).collect::<Vec<_>>().try_into().unwrap();
        let noises = super::SurfaceNoises::new(&crate::random::Xoroshiro::from_seed(20261003).fork_positional(), 20261003);
        let mut scale = 1.0f32;
        for octave in &noises.depth {
            let [xo, zo] = octave.origin;
            let (px, pz) = (x as f32 * scale * 0.125 + xo, z as f32 * scale * 0.125 + zo);
            let value = octave.value(px, pz);
            println!("octave input ({px}, {pz}) value {value} {:08x}", value.to_bits());
            scale *= 0.5;
        }
        println!("depth noise {}", noises.depth(x, z));
    }
}
