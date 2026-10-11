//! The structure stage across chunks (`structurePostProcessChunk`, after carvers and before
//! decoration): every chunk a start reaches is post-processed once, reading the 3x3 chunks around
//! it as the post-processes before it left them and writing into them at once. The order is the
//! canonical one or a logged run's (order.rs); the mechanics are the same for both
//! (docs/research/worldgen-bds-order.md, "One engine").

mod around;
pub(crate) mod order;
mod probe;
pub(crate) mod replay;

use std::sync::Arc;

use crate::feature;
use crate::structure::{self, Past, Settled, Spawners, trace};
use crate::terrain::ChunkShape;
use crate::OverworldGenerator;
use around::Around;
use order::Rank;

type Pos = (i32, i32, i32);

/// A chunk at one moment of the stage.
#[derive(Clone)]
pub struct Pieced {
    shape: Arc<ChunkShape>,
    /// Its cells a template pass decided: `rejoin_connections` leaves them.
    settled: Arc<Settled>,
    /// The heights written so far, for `rejoin_connections`.
    touched: Option<(i32, i32)>,
}

/// One chunk's post-process.
pub struct Source {
    /// The chunk after it.
    own: Pieced,
    /// Its writes into the chunks around, in order.
    past: Vec<Past>,
    /// The cells around its chunk that its passes decided (true) or its pieces wrote afterwards.
    settled_past: Vec<(Pos, bool)>,
    /// The spider corridors whose spawner is placed by its end.
    spawners: Spawners,
    /// The height of each desert pyramid it placed, by start.
    temple_heights: Vec<((i32, i32), i32)>,
}

fn widen(heights: Option<(i32, i32)>, (low, high): (i32, i32)) -> Option<(i32, i32)> {
    Some(heights.map_or((low, high), |(lo, hi)| (lo.min(low), hi.max(high))))
}

fn settle(settled: &mut Arc<Settled>, at: Pos, decided: bool) {
    if decided {
        Arc::make_mut(settled).insert(at);
    } else if settled.contains(&at) {
        Arc::make_mut(settled).remove(&at);
    }
}

fn square((x, z): (i32, i32), reach: i32) -> impl Iterator<Item = (i32, i32)> {
    (x - reach..=x + reach).flat_map(move |nx| (z - reach..=z + reach).map(move |nz| (nx, nz)))
}

impl Source {
    /// `before`, a chunk beside the source's, with the source's writes into it.
    fn landed(&self, before: &Pieced, chunk: (i32, i32)) -> Pieced {
        let mut after = before.clone();
        for &([x, y, z], block) in self.past.iter().filter(|(p, _)| (p[0] >> 4, p[2] >> 4) == chunk) {
            Arc::make_mut(&mut after.shape).set_block((x & 15) as usize, y, (z & 15) as usize, block);
            after.touched = widen(after.touched, (y, y));
        }
        for &(at, decided) in self.settled_past.iter().filter(|(p, _)| (p.0 >> 4, p.2 >> 4) == chunk) {
            settle(&mut after.settled, at, decided);
        }
        after
    }
}

impl OverworldGenerator {
    /// `shape`, the chunk after its carvers, becomes the chunk after the structure stage.
    pub fn place_structures(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        self.warm_structures();
        let chunk = (chunk_x, chunk_z);
        let staged = self.staged_after(chunk);
        *shape = (*staged.shape).clone();
        if let Some(heights) = staged.touched {
            let load = |n: (i32, i32)| self.staged_after(n);
            let around = Around::new(chunk, &load);
            let neighbour = |x: i32, y: i32, z: i32| around.get((x >> 4, z >> 4)).shape.block((x & 15) as usize, y, (z & 15) as usize);
            trace::label(|| "rejoin".into());
            structure::rejoin_connections(shape, chunk, heights, &neighbour, &staged.settled);
        }
    }

    /// The chunks whose post-processes write `chunk` (itself among them), in order.
    fn writers(&self, chunk: (i32, i32)) -> Vec<(Rank, (i32, i32))> {
        let mut writers: Vec<_> = square(chunk, 1).filter_map(|n| self.structure_rank(n).map(|rank| (rank, n))).collect();
        writers.sort_unstable();
        writers
    }

    /// `chunk` as the post-processes ranked below `limit` left it.
    fn staged_before(&self, chunk: (i32, i32), limit: Rank) -> Arc<Pieced> {
        let writers = self.writers(chunk);
        let earlier = writers.iter().take_while(|(rank, _)| *rank < limit).count();
        self.staged(chunk, &writers[..earlier])
    }

    /// `chunk` as the structure stage leaves it, before `rejoin_connections`.
    fn staged_after(&self, chunk: (i32, i32)) -> Arc<Pieced> {
        self.staged(chunk, &self.writers(chunk))
    }

    /// `chunk` after `writers`, the first of those that write it.
    fn staged(&self, chunk: (i32, i32), writers: &[(Rank, (i32, i32))]) -> Arc<Pieced> {
        self.staged.get((chunk.0, chunk.1, writers.len() as u8), || {
            let Some((&(_, writer), earlier)) = writers.split_last() else {
                return Pieced { shape: self.carved(chunk.0, chunk.1), settled: Default::default(), touched: None };
            };
            let before = self.staged(chunk, earlier);
            match &*self.source(writer) {
                None => (*before).clone(),
                Some(source) if writer == chunk => source.own.clone(),
                Some(source) => source.landed(&before, chunk),
            }
        })
    }

    /// None: no start reaches the chunk, or a logged run did not post-process it.
    fn source(&self, chunk: (i32, i32)) -> Arc<Option<Source>> {
        self.pieced.get(chunk, || self.post_process(chunk))
    }

    /// The post-processes within `reach` chunks of `chunk` that ran before the one ranked `rank`.
    fn earlier_sources(&self, chunk: (i32, i32), rank: Rank, reach: i32) -> impl Iterator<Item = Arc<Option<Source>>> + '_ {
        square(chunk, reach).filter(move |&n| self.structure_rank(n).is_some_and(|r| r < rank)).map(|n| self.source(n))
    }

    fn post_process(&self, chunk: (i32, i32)) -> Option<Source> {
        let rank = self.structure_rank(chunk)?;
        let reaching = self.structures.reaching(&self.density, chunk)?;
        let before = |n: (i32, i32)| self.staged_before(n, rank);
        let around = Around::new(chunk, &before);
        let Pieced { shape, mut settled, touched } = (**around.get(chunk)).clone();
        let mut shape = Arc::unwrap_or_clone(shape);
        let block = |x: i32, y: i32, z: i32| around.get((x >> 4, z >> 4)).shape.block((x & 15) as usize, y, (z & 15) as usize);
        // A corridor is at most three chunks long.
        let spawners = || self.earlier_sources(chunk, rank, 2).flat_map(|source| (*source).as_ref().map_or_else(Spawners::new, |s| s.spawners.clone())).collect();
        let tops = || self.generated_tops(chunk.0, chunk.1);
        let shaft_order = || self.shaft_order(chunk);
        let taken = |source: Arc<Option<Source>>, start: (i32, i32)| (*source).as_ref()?.temple_heights.iter().find(|(s, _)| *s == start).map(|&(_, y)| y);
        let temple_height = |start: (i32, i32)| self.earlier_sources(chunk, rank, 1).find_map(|source| taken(source, start));
        let neighbours = structure::Neighbours { placed: &block, spawners: &spawners, generated_tops: &tops, shaft_order: &shaft_order, temple_height: &temple_height, pyramid_from_start: self.structure_order.is_canonical() };
        // Pieces' features read the same state; a feature's region is the 3x3.
        let load = |x: i32, z: i32| if around.holds((x, z)) { around.get((x, z)).shape.clone() } else { self.carved(x, z) };
        let biome = |[x, y, z]: [i32; 3]| Some(self.chunk_biomes(x >> 4, z >> 4).biome((x & 15) as usize, y, (z & 15) as usize));
        let surface = |qx: i32, qz: i32| self.density.preliminary_surface(qx, qz);
        let features = feature::ElementWorld { load: &load, biome: &biome, surface: &surface };
        let biomes = self.chunk_biomes(chunk.0, chunk.1);
        let placed = self.structures.place(&self.density, &reaching, &mut shape, chunk, neighbours, &biomes, &features);
        let mut settled_past = Vec::new();
        for (at, decided) in placed.settled {
            if (at.0 >> 4, at.2 >> 4) == chunk {
                settle(&mut settled, at, decided);
            } else {
                settled_past.push((at, decided));
            }
        }
        let own = Pieced { shape: Arc::new(shape), settled, touched: widen(touched, placed.touched) };
        Some(Source { own, past: placed.past, settled_past, spawners: placed.spawners, temple_heights: placed.temple_heights })
    }
}
