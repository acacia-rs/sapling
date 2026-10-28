//! The End's structure stage (`TheEndGenerator::structurePostProcessChunk` 148244bd0): its one feature, the end city
//! (structure/end_city), on the chunk's large-feature generator as the overworld seeds it.

use std::sync::Arc;

use super::canvas::{Canvas, Spawners};
use super::end_city::{self, EndCity};
use super::stage::Placed;
use super::start_cache::StartCache;
use crate::block::Block;
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::ChunkShape;

pub use super::end_city::GRID;

const STARTS_LIMIT: usize = 256;
pub const TOP: i32 = 255;

/// The End's terrain and island height of a chunk, which the start test reads.
pub struct StartInputs<'a> {
    pub terrain: &'a dyn Fn(i32, i32) -> Arc<ChunkShape>,
    pub island: &'a dyn Fn(i32, i32) -> f32,
}

pub struct EndStructures {
    level_seed: u32,
    cities: StartCache<(i32, i32), Option<EndCity>>,
}

impl EndStructures {
    pub fn new(seed: i64) -> Self {
        Self { level_seed: seed as u32, cities: StartCache::new(STARTS_LIMIT) }
    }

    /// The city of its region if it starts in `start`.
    pub fn city(&self, start: (i32, i32), inputs: &StartInputs) -> Option<Arc<Option<EndCity>>> {
        let (candidate, mut random) = end_city::GRID.candidate_random(self.level_seed, start);
        if candidate != start {
            return None;
        }
        let city = self.cities.get(start, || {
            let terrain = (inputs.terrain)(start.0, start.1);
            end_city::is_start(start, (inputs.island)(start.0, start.1), &terrain).then(|| end_city::generate(&mut random, start, &terrain))
        });
        city.is_some().then_some(city)
    }

    /// The cities whose pieces meet `chunk`; None without any.
    pub fn reaching(&self, chunk: (i32, i32), inputs: &StartInputs) -> Option<Vec<Arc<Option<EndCity>>>> {
        let column = super::BoundingBox::chunk(chunk.0, chunk.1);
        let spacing = end_city::GRID.spacing;
        let regions = |c: i32| (c - end_city::REACH).div_euclid(spacing)..=(c + end_city::REACH).div_euclid(spacing);
        let mut cities = Vec::new();
        for rx in regions(chunk.0) {
            for rz in regions(chunk.1) {
                let start = end_city::GRID.candidate(self.level_seed, (rx * spacing, rz * spacing));
                if (start.0 - chunk.0).abs() > end_city::REACH || (start.1 - chunk.1).abs() > end_city::REACH {
                    continue;
                }
                if let Some(city) = self.city(start, inputs).filter(|c| c.as_ref().as_ref().is_some_and(|c| c.bb.intersects(&column))) {
                    cities.push(city);
                }
            }
        }
        (!cities.is_empty()).then_some(cities)
    }

    /// The post-process of `chunk` over `shape` (`neighbour` reads the eight chunks around).
    pub fn place(&self, cities: &[Arc<Option<EndCity>>], shape: &mut ChunkShape, chunk: (i32, i32), neighbour: &dyn Fn(i32, i32, i32) -> Block) -> Placed {
        let mut canvas = Canvas::new(shape, chunk, neighbour).with_heights(0, TOP);
        let mut touched = (i32::MAX, i32::MIN);
        let mut random = Mt19937::new(LargeFeatureSeed::new(self.level_seed).chunk(chunk.0, chunk.1));
        for city in cities.iter().filter_map(|c| c.as_ref().as_ref()) {
            city.post_process(&mut canvas, &mut random);
            touched = (touched.0.min(city.bb.y0 - 1), touched.1.max(city.bb.y1 + 1));
        }
        let touched = (touched.0.min(canvas.touched.0), touched.1.max(canvas.touched.1));
        let mut past = std::mem::take(&mut canvas.unclipped);
        past.append(&mut canvas.spilled);
        Placed { touched, past, past_entities: std::mem::take(&mut canvas.past_entities), spawners: Spawners::new(), settled: canvas.record.take_settled(), temple_heights: Vec::new() }
    }
}
