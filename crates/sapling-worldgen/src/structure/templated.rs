//! The template structures' part of the structure stage. A scattered start (igloo) post-processes
//! once, in its start chunk, and places through BDS's 3x3 block source, so a chunk gets the blocks
//! of every igloo within a chunk of it, each drawn with its own start chunk's generator. Template
//! pieces (ocean ruins) post-process in every chunk their box meets, clipped to that chunk.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::canvas::Canvas;
use super::kind::MONUMENT;
use super::probe::Probe;
use super::{BoundingBox, StructureKind, Structures, igloo, ocean_ruin};
use crate::density::Density;
use crate::random::{LargeFeatureSeed, Mt19937};

/// Ruin pieces stay within the big ruin's chunk and the cluster spots up to 32 blocks around it.
const OCEAN_RUIN_REACH: i32 = 3;

#[derive(Default)]
pub(super) struct TemplateStarts {
    ocean_ruins: Mutex<HashMap<(i32, i32), Arc<ocean_ruin::OceanRuin>>>,
}

impl Structures {
    fn centre_biome(density: &Density, (x, z): (i32, i32)) -> u32 {
        let probe = Probe { density };
        probe.biome(x * 16 + 8, probe.surface((x, z)), z * 16 + 8)
    }

    /// Igloo starts whose templates may reach `chunk`.
    pub(super) fn igloos_near(&self, density: &Density, chunk: (i32, i32)) -> Vec<(i32, i32)> {
        let starts = self.starts_near(density, StructureKind::Temple, chunk, 1);
        starts.into_iter().filter(|&s| igloo::IGLOO_BIOMES.contains(&Self::centre_biome(density, s))).collect()
    }

    /// The scattered feature's igloos (the pyramids and huts are hardcoded pieces).
    pub(super) fn place_igloos(&self, canvas: &mut Canvas, igloos: &[(i32, i32)]) {
        let seeds = LargeFeatureSeed::new(self.level_seed);
        for &start in igloos {
            igloo::place(canvas, &mut Mt19937::new(seeds.chunk(start.0, start.1)), start);
        }
    }

    /// The ocean ruins with a piece meeting `chunk`.
    pub(super) fn ocean_ruins_near(&self, density: &Density, chunk: (i32, i32)) -> Vec<Arc<ocean_ruin::OceanRuin>> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let starts = self.starts_near(density, StructureKind::OceanRuins, chunk, OCEAN_RUIN_REACH);
        starts
            .into_iter()
            .map(|start| self.ocean_ruin(density, start))
            .filter(|ruin| ruin.pieces.iter().any(|p| p.bb.intersects(&column)))
            .collect()
    }

    fn ocean_ruin(&self, density: &Density, start: (i32, i32)) -> Arc<ocean_ruin::OceanRuin> {
        let mut cache = self.templates.ocean_ruins.lock().unwrap();
        cache
            .entry(start)
            .or_insert_with(|| {
                let warm = ocean_ruin::WARM_OCEANS.contains(&Self::centre_biome(density, start));
                Arc::new(ocean_ruin::generate(self.monument_checked_random(start), start, warm))
            })
            .clone()
    }

    /// The blueprint generator of a start whose check ends with the monument test over chunks
    /// x-5..x+4, z-5..z+4 (ocean ruins, shipwrecks): each test reseeds it for its grid region,
    /// the last for (x+4, z+4).
    fn monument_checked_random(&self, (x, z): (i32, i32)) -> Mt19937 {
        MONUMENT.candidate_random(self.level_seed, (x + 4, z + 4)).1
    }

    /// Every ruin piece meeting the canvas's chunk; `seed` is the chunk's structure seed.
    pub(super) fn place_ocean_ruins(canvas: &mut Canvas, ruins: &[Arc<ocean_ruin::OceanRuin>], seed: u32) {
        let column = canvas.bb;
        for piece in ruins.iter().flat_map(|r| r.pieces.iter()).filter(|p| p.bb.intersects(&column)) {
            piece.post_process(canvas, seed);
        }
    }
}
