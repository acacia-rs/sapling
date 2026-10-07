//! The template structures' part of the structure stage. Their starts post-process once, in the
//! start chunk, and place templates through BDS's 3x3 block source, so a chunk gets the blocks
//! of every such start within a chunk of it, each drawn with its own start chunk's generator.

use super::canvas::Canvas;
use super::kind::TEMPLE_BIOMES;
use super::probe::Probe;
use super::{StructureKind, Structures, igloo};
use crate::density::Density;
use crate::random::{LargeFeatureSeed, Mt19937};

impl Structures {
    /// Igloo starts whose templates may reach `chunk`.
    pub(super) fn igloos_near(&self, density: &Density, chunk: (i32, i32)) -> Vec<(i32, i32)> {
        let probe = Probe { density };
        let biome = |(x, z): (i32, i32)| probe.biome(x * 16 + 8, probe.surface((x, z)), z * 16 + 8);
        debug_assert!(igloo::IGLOO_BIOMES.iter().all(|b| TEMPLE_BIOMES.contains(b)));
        self.starts_near(density, StructureKind::Temple, chunk, 1).into_iter().filter(|&s| igloo::IGLOO_BIOMES.contains(&biome(s))).collect()
    }

    /// The scattered feature's igloos (the pyramids and huts are hardcoded pieces).
    pub(super) fn place_igloos(&self, canvas: &mut Canvas, igloos: &[(i32, i32)]) {
        let seeds = LargeFeatureSeed::new(self.level_seed);
        for &start in igloos {
            igloo::place(canvas, &mut Mt19937::new(seeds.chunk(start.0, start.1)), start);
        }
    }
}
