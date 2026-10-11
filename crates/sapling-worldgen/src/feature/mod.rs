//! BDS's data-driven feature rules (`feature_rules`/`features` JSON, transcribed into data/):
//! each source chunk is decorated alone over its neighbours' terrain, and a column replays the
//! writes of every source chunk that reaches it (docs/research/worldgen-bds.md, Feature placement).

mod aquatic;
mod attach;
mod blob;
mod coral;
mod cave;
mod cave_spec;
mod data;
mod decorate;
mod disk;
mod element;
mod flowers;
mod fossil;
mod freeze;
mod geode;
mod ice_spike;
mod iceberg;
mod jungle;
mod lake;
mod land;
mod huge_mushroom;
mod legacy_spec;
mod legacy_tree;
mod liquid;
mod lush;
mod molang;
mod monster_room;
mod multiface;
mod order;
mod ore;
mod pale;
mod pile;
mod place;
mod plants;
mod pool;
pub(crate) mod reach;
mod scatter;
mod sculk;
mod spec;
mod speleothem;
mod spreader;
mod springs;
mod thread;
mod ticks;
pub(crate) mod trace;
mod tree;
mod view;

use std::collections::HashMap;
use std::sync::Arc;

use crate::biome::ChunkBiomes;
use crate::cache::SourceCache;
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::ChunkShape;

pub(crate) use element::{ElementWorld, element_random, place_element};
pub(crate) use place::swap_feature;
use view::{ChunkWrites, REACH, View};

/// Source chunks' writes are a few hundred kB each.
const CACHE_LIMIT: usize = 1024;
/// Chunks as earlier phases left them, about 200 kB each.
const BEFORE_LIMIT: usize = 768;

type Load<'a> = &'a dyn Fn(i32, i32) -> Arc<ChunkShape>;
type Biomes<'a> = &'a dyn Fn(i32, i32) -> Arc<ChunkBiomes>;

/// What decoration reads of the world besides the view: chunks' terrain before features, their
/// biomes, and the preliminary surface level of a quart column.
#[derive(Clone, Copy)]
pub struct Inputs<'a> {
    pub loaded: Load<'a>,
    /// Chunks as `loadChunk` left them, before the structure stage.
    pub carved: Load<'a>,
    /// Chunks' pre-worldgen heightmaps (`ChunkShape::tops` before the surface and carvers).
    pub generated_tops: &'a dyn Fn(i32, i32) -> Arc<[i16; 256]>,
    pub biomes: Biomes<'a>,
    pub surface: &'a dyn Fn(i32, i32) -> i32,
    /// Whether a structure holds the block (geode.rs).
    pub in_structure: &'a dyn Fn([i32; 3]) -> bool,
    /// Whether a mineshaft's or a stronghold's box holds the block (multiface.rs).
    pub in_tunnels: &'a dyn Fn([i32; 3]) -> bool,
}

pub(crate) fn query_noise(x: f32, z: f32) -> f32 {
    molang::noise().value(x, z)
}

/// When BDS decorates a chunk relative to its neighbours: x and z even first, then x odd, both odd,
/// z odd. Chunks of one phase are never neighbours (docs/research/worldgen-bds-features.md).
pub(crate) fn phase((x, z): (i32, i32)) -> u8 {
    [0, 1, 3, 2][((x & 1) | (z & 1) << 1) as usize]
}

/// A source's place in the decoration order: its phase, or its index in a run's logged order.
type Rank = u32;

pub struct Features {
    seed: LargeFeatureSeed,
    writes: SourceCache<(i32, i32), ChunkWrites>,
    before: SourceCache<(i32, i32, Rank), ChunkShape>,
    /// Sources follow what BDS's thread-local generator could reach (reach.rs).
    pub track_reach: bool,
    /// The order one BDS run decorated its chunks in, replacing the phases: every source then sees
    /// the writes of all before it, and chunks the run did not decorate are no sources.
    pub order: Option<HashMap<(i32, i32), Rank>>,
    /// With `order`, for a one-worker run: its thread generator's seed and, per rank, the draws the
    /// generator had spent at that chunk's first `MultifaceFeature` draw (thread.rs).
    thread: Option<(u32, Vec<Option<u32>>)>,
}

impl Features {
    /// Seeded like the carvers: `(x*a + z*b) ^ seed` on the low 32 bits of the world seed.
    pub fn new(world_seed: i64) -> Self {
        Self { seed: LargeFeatureSeed::new(world_seed as u32), writes: SourceCache::new(CACHE_LIMIT), before: SourceCache::new(BEFORE_LIMIT), track_reach: false, order: None, thread: None }
    }

    /// Decorates in `order`, every source over all earlier ones; `thread`: the seed of the one worker's
    /// generator in the BDS run the order comes from and its draws spent per rank (see the field).
    pub fn set_order(&mut self, order: &[(i32, i32)], thread: Option<(u32, &[Option<u32>])>) {
        self.order = Some(order.iter().enumerate().map(|(rank, &chunk)| (chunk, rank as Rank)).collect());
        self.thread = thread.map(|(seed, spent)| (seed, spent.to_vec()));
    }

    /// The thread generator as the run had it when the source ranked `rank` first drew from it.
    fn thread_before(&self, rank: Rank) -> Option<Mt19937> {
        let (seed, spent) = self.thread.as_ref()?;
        let mut random = Mt19937::new(*seed);
        random.consume_count(spent[rank as usize].unwrap_or(0));
        Some(random)
    }

    fn rank(&self, source: (i32, i32)) -> Option<Rank> {
        match &self.order {
            Some(order) => order.get(&source).copied(),
            None => Some(phase(source) as Rank),
        }
    }

    /// The sources whose features reach `target`, in decoration order, those ranked `limit` and later left out.
    fn sources_before(&self, target: (i32, i32), limit: Rank) -> Vec<(i32, i32)> {
        let mut sources: Vec<(Rank, usize, (i32, i32))> = (target.0 - REACH..=target.0 + REACH)
            .flat_map(|x| (target.1 - REACH..=target.1 + REACH).map(move |z| (x, z)))
            .filter_map(|source| self.rank(source).filter(|&rank| rank < limit).map(|rank| (rank, order::rank(source), source)))
            .collect();
        sources.sort();
        sources.into_iter().map(|(.., source)| source).collect()
    }

    /// The cells of `chunk` that sources decorated before `limit` left in doubt.
    fn doubt_before(&self, chunk: (i32, i32), limit: Rank, inputs: Inputs) -> Option<reach::Cells> {
        let mut cells: Option<reach::Cells> = None;
        for source in self.sources_before(chunk, limit) {
            if let Some(doubt) = self.source_writes(source, inputs).doubt_towards(chunk.0 - source.0, chunk.1 - source.1) {
                cells.get_or_insert_with(Default::default).merge(doubt);
            }
        }
        cells
    }

    /// What every source left in doubt in `chunk`, and the tries `chunk` followed as a source.
    pub fn reach(&self, chunk: (i32, i32), inputs: Inputs) -> (Option<reach::Cells>, reach::Tries) {
        let tries = self.rank(chunk).map_or_else(Default::default, |_| self.source_writes(chunk, inputs).tries());
        (self.doubt_before(chunk, Rank::MAX, inputs), tries)
    }

    /// Sources apply in decoration order; a write lands when the block it replaced in its source's
    /// view is still there.
    pub fn place(&self, shape: &mut ChunkShape, chunk: (i32, i32), inputs: Inputs) {
        let arms: Vec<(usize, crate::block::Block)> = (0..16 * 16 * crate::terrain::HEIGHT).map(|index| (index, shape.block_at(index))).filter(|&(_, block)| spreader::is_piston_arm(block)).collect();
        self.replay(shape, chunk, Rank::MAX, inputs);
        arms.into_iter().for_each(|(index, arm)| shape.set_at(index, arm));
    }

    fn replay(&self, shape: &mut ChunkShape, chunk: (i32, i32), limit: Rank, inputs: Inputs) {
        for source in self.sources_before(chunk, limit) {
            self.apply(shape, chunk, source, inputs);
        }
    }

    fn apply(&self, shape: &mut ChunkShape, chunk: (i32, i32), source: (i32, i32), inputs: Inputs) {
        let writes = self.source_writes(source, inputs);
        for write in writes.towards(chunk.0 - source.0, chunk.1 - source.1) {
            let index = write.index as usize;
            if shape.block_at(index) == write.seen {
                shape.set_at(index, write.block);
            }
        }
    }

    /// A chunk with the features of every source decorated before `limit`.
    fn before(&self, chunk: (i32, i32), limit: Rank, inputs: Inputs) -> Arc<ChunkShape> {
        if limit == 0 {
            return (inputs.loaded)(chunk.0, chunk.1);
        }
        self.before.get((chunk.0, chunk.1, limit), || {
            let mut shape = (*(inputs.loaded)(chunk.0, chunk.1)).clone();
            self.replay(&mut shape, chunk, limit, inputs);
            shape
        })
    }

    /// One source chunk's writes over its neighbours as the sources before it left them.
    pub fn source_writes(&self, source: (i32, i32), inputs: Inputs) -> Arc<ChunkWrites> {
        self.writes.get(source, || {
            let limit = self.rank(source).expect("a source the order holds");
            // The same-phase diagnostic (order.rs) is for the phase model only.
            let earlier = if self.order.is_none() { order::earlier(source) } else { Vec::new() };
            let base = |x: i32, z: i32| {
                let shape = self.before((x, z), limit, inputs);
                let reach = |e: &&(i32, i32)| (e.0 - x).abs() <= REACH && (e.1 - z).abs() <= REACH;
                if !earlier.iter().any(|e| reach(&e)) {
                    return shape;
                }
                let mut shape = (*shape).clone();
                for &from in earlier.iter().filter(reach) {
                    self.apply(&mut shape, (x, z), from, inputs);
                }
                Arc::new(shape)
            };
            // No chunk's heightmap changes while the world generates: see docs/research/worldgen-bds-features.md, "Heightmaps".
            let frozen = inputs.carved;
            // The pre-worldgen heightmap is dropped when the chunk's decoration ends.
            let generated = |x: i32, z: i32| self.rank((x, z)).is_none_or(|rank| rank >= limit).then(|| (inputs.generated_tops)(x, z));
            let mut view = View::new(source, &base).with_frozen_heights(frozen).with_generated_tops(&generated);
            let doubt = |x: i32, z: i32| self.doubt_before((x, z), limit, inputs);
            view.reach = self.track_reach.then(|| reach::Reach::new(&doubt));
            view.thread_random = self.thread_before(limit);
            let mut post_process = Mt19937::new(self.seed.chunk(source.0, source.1));
            lake::lava_lake(&mut post_process, source, &mut view);
            monster_room::monster_rooms(post_process, source, &mut view);
            decorate::decorate_chunk(&self.seed, source, inputs, &mut view);
            freeze::freeze_chunk(source, &(inputs.biomes)(source.0, source.1), &mut view);
            let inherited: Vec<_> = self.sources_before(source, limit).into_iter().flat_map(|n| self.source_writes(n, inputs).routed_to(source).collect::<Vec<_>>()).collect();
            liquid::settle(&mut view, &inherited);
            view.into_writes()
        })
    }
}
