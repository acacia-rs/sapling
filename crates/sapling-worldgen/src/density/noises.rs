//! The named noises the density stage reads, with Java 1.18.1's parameters as BDS builds them
//! (`OverworldGeneratorMultinoise::Noises::make` 148220f20, `CavifierNoises::make` 148234150).

use crate::noise::NormalNoise;
use crate::random::PositionalFactory;

fn noise(factory: &PositionalFactory, name: &str, first_octave: i32, amplitudes: &[f32]) -> NormalNoise {
    NormalNoise::new(factory, &format!("minecraft:{name}"), first_octave, amplitudes)
}

#[derive(Debug, Clone)]
pub struct CaveNoises {
    pub pillar: NormalNoise,
    pub pillar_rareness: NormalNoise,
    pub pillar_thickness: NormalNoise,
    pub spaghetti_2d: NormalNoise,
    pub spaghetti_2d_elevation: NormalNoise,
    pub spaghetti_2d_modulator: NormalNoise,
    pub spaghetti_2d_thickness: NormalNoise,
    pub spaghetti_3d_1: NormalNoise,
    pub spaghetti_3d_2: NormalNoise,
    pub spaghetti_3d_rarity: NormalNoise,
    pub spaghetti_3d_thickness: NormalNoise,
    pub spaghetti_roughness: NormalNoise,
    pub spaghetti_roughness_modulator: NormalNoise,
    pub cave_entrance: NormalNoise,
    pub cave_layer: NormalNoise,
    pub cave_cheese: NormalNoise,
}

impl CaveNoises {
    pub fn new(factory: &PositionalFactory) -> Self {
        let one = |name, octave| noise(factory, name, octave, &[1.0]);
        Self {
            pillar: noise(factory, "pillar", -7, &[1.0, 1.0]),
            pillar_rareness: one("pillar_rareness", -8),
            pillar_thickness: one("pillar_thickness", -8),
            spaghetti_2d: one("spaghetti_2d", -7),
            spaghetti_2d_elevation: one("spaghetti_2d_elevation", -8),
            spaghetti_2d_modulator: one("spaghetti_2d_modulator", -11),
            spaghetti_2d_thickness: one("spaghetti_2d_thickness", -11),
            spaghetti_3d_1: one("spaghetti_3d_1", -7),
            spaghetti_3d_2: one("spaghetti_3d_2", -7),
            spaghetti_3d_rarity: one("spaghetti_3d_rarity", -11),
            spaghetti_3d_thickness: one("spaghetti_3d_thickness", -8),
            spaghetti_roughness: one("spaghetti_roughness", -5),
            spaghetti_roughness_modulator: one("spaghetti_roughness_modulator", -8),
            cave_entrance: noise(factory, "cave_entrance", -7, &[0.4, 0.5, 1.0]),
            cave_layer: one("cave_layer", -8),
            cave_cheese: noise(factory, "cave_cheese", -8, &[0.5, 1.0, 2.0, 1.0, 2.0, 1.0, 0.0, 2.0, 0.0]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoodleNoises {
    pub toggle: NormalNoise,
    pub thickness: NormalNoise,
    pub ridge_a: NormalNoise,
    pub ridge_b: NormalNoise,
}

impl NoodleNoises {
    pub fn new(factory: &PositionalFactory) -> Self {
        Self {
            toggle: noise(factory, "noodle", -8, &[1.0]),
            thickness: noise(factory, "noodle_thickness", -8, &[1.0]),
            ridge_a: noise(factory, "noodle_ridge_a", -7, &[1.0]),
            ridge_b: noise(factory, "noodle_ridge_b", -7, &[1.0]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AquiferNoises {
    pub barrier: NormalNoise,
    pub floodedness: NormalNoise,
    pub lava: NormalNoise,
    pub spread: NormalNoise,
    /// Places each aquifer cell's center.
    pub positional: PositionalFactory,
}

impl AquiferNoises {
    pub fn new(factory: &PositionalFactory) -> Self {
        Self {
            barrier: noise(factory, "aquifer_barrier", -3, &[1.0]),
            floodedness: noise(factory, "aquifer_fluid_level_floodedness", -7, &[1.0]),
            lava: noise(factory, "aquifer_lava", -1, &[1.0]),
            spread: noise(factory, "aquifer_fluid_level_spread", -5, &[1.0]),
            positional: factory.from_hash_of("minecraft:aquifer").fork_positional(),
        }
    }
}
