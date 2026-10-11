//! The structure stage as one logged BDS run did it (scripts/dbg_loads.py `--structure-order`):
//! chunks post-processed in that run's order, each reading its neighbours as they are at that
//! moment and writing past its box at once, mineshaft starts visited in the order the run's
//! chunk loads filed them: docs/research/worldgen-bds-order.md, "Replaying a run".

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use super::Pieced;
use crate::block::Block;
use crate::structure::{self, Spawners, StartMap, StructureKind, trace};
use crate::terrain::ChunkShape;
use crate::{OverworldGenerator, feature};

/// What the run did to a chunk, in its log's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureStep {
    /// `loadChunk`: the structure features scan for starts around it.
    Load((i32, i32)),
    /// `postProcessStructureFeatures`.
    Post((i32, i32)),
}

/// A write of the replayed stage, in order.
#[derive(Debug, Clone)]
pub struct ReplayWrite {
    /// The writing chunk's place among the run's post-processed chunks.
    pub rank: usize,
    pub chunk: (i32, i32),
    pub at: [i32; 3],
    pub block: Block,
    /// What wrote it (structure/trace.rs).
    pub label: Arc<str>,
}

struct Done {
    chunks: HashMap<(i32, i32), Arc<Pieced>>,
    writes: Vec<ReplayWrite>,
}

pub struct Replay {
    steps: Vec<StructureStep>,
    done: OnceLock<Done>,
}

type Shapes = HashMap<(i32, i32), ChunkShape>;

impl OverworldGenerator {
    /// Runs the structure stage as one BDS run did; a chunk's second `Post` is dropped. Without
    /// `Load` steps mineshaft starts keep our own order.
    #[doc(hidden)]
    pub fn set_structure_order(&mut self, steps: &[StructureStep]) {
        let mut seen = HashSet::new();
        let once = |step: &&StructureStep| !matches!(**step, StructureStep::Post(chunk) if !seen.insert(chunk));
        self.structure_replay = Some(Replay { steps: steps.iter().filter(once).copied().collect(), done: OnceLock::new() });
    }

    /// Every write of the replayed structure stage, in the run's order; empty without `set_structure_order`.
    #[doc(hidden)]
    pub fn structure_replay_writes(&self) -> &[ReplayWrite] {
        self.structure_replay.as_ref().map_or(&[], |replay| &replay.done.get_or_init(|| self.run_replay(&replay.steps)).writes)
    }

    pub(super) fn replayed(&self, replay: &Replay, chunk: (i32, i32)) -> Arc<Pieced> {
        let done = replay.done.get_or_init(|| self.run_replay(&replay.steps));
        done.chunks.get(&chunk).cloned().unwrap_or_else(|| Arc::new(Pieced::bare(self.carved(chunk.0, chunk.1))))
    }

    /// The carved terrain of `chunks`, generated on every core.
    fn carved_all(&self, chunks: &[(i32, i32)]) -> Shapes {
        let (next, found) = (AtomicUsize::new(0), Mutex::new(Shapes::new()));
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| {
                    while let Some(&(x, z)) = chunks.get(next.fetch_add(1, Ordering::Relaxed)) {
                        let shape = (*self.carved(x, z)).clone();
                        found.lock().unwrap().insert((x, z), shape);
                    }
                });
            }
        });
        found.into_inner().unwrap()
    }

    fn run_replay(&self, steps: &[StructureStep]) -> Done {
        let around = |(x, z): (i32, i32)| (x - 1..=x + 1).flat_map(move |nx| (z - 1..=z + 1).map(move |nz| (nx, nz)));
        let posts = steps.iter().filter_map(|step| if let StructureStep::Post(chunk) = step { Some(*chunk) } else { None });
        let mut wanted: Vec<(i32, i32)> = posts.flat_map(around).collect();
        wanted.sort_unstable();
        wanted.dedup();
        let state = RefCell::new(self.carved_all(&wanted));
        let mut spawners = Spawners::new();
        let mut touched: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
        let mut writes = Vec::new();
        // Pieces' features land after every chunk's pieces, as without a replay (structured.rs).
        let mut spilled = HashMap::new();
        let mut starts = StartMap::default();
        let logged_loads = steps.iter().any(|step| matches!(step, StructureStep::Load(_)));
        // TODO: an experiment's switch; settle and drop (villages, strongholds, mansions, cities read the state).
        let live_kinds = std::env::var_os("SAPLING_REPLAY_LIVE_KINDS").is_some();
        let mut rank = 0;
        for step in steps {
            let chunk = match *step {
                StructureStep::Load(chunk) => {
                    starts.load(chunk, &|start| self.structures.is_start(&self.density, StructureKind::Mineshaft, start));
                    continue;
                }
                StructureStep::Post(chunk) => chunk,
            };
            let mut shape = state.borrow_mut().remove(&chunk).expect("a chunk posted once");
            let shaft_order = logged_loads.then(|| starts.order());
            let (placed, traced) = {
                let block = |x: i32, y: i32, z: i32| state.borrow().get(&(x >> 4, z >> 4)).map_or(Block::Air, |s| s.block((x & 15) as usize, y, (z & 15) as usize));
                let placed_spawners = || spawners.clone();
                let tops = || self.generated_tops(chunk.0, chunk.1);
                // Only mineshafts read the run's state: the other kinds' pieces are placed per chunk from the
                // start's generator over carved terrain (structure/shipwreck.rs), which a moving state breaks.
                let carved = |x: i32, y: i32, z: i32| self.carved(x >> 4, z >> 4).block((x & 15) as usize, y, (z & 15) as usize);
                let neighbours =
                    structure::Neighbours { carved: &carved, placed: &block, spawners: &placed_spawners, generated_tops: &tops, shaft_order: shaft_order.as_deref(), live: live_kinds };
                let load = |x: i32, z: i32| self.carved(x, z);
                let biome = |[x, y, z]: [i32; 3]| Some(self.chunk_biomes(x >> 4, z >> 4).biome((x & 15) as usize, y, (z & 15) as usize));
                let surface = |qx: i32, qz: i32| self.density.preliminary_surface(qx, qz);
                let features = feature::ElementWorld { load: &load, biome: &biome, surface: &surface };
                let biomes = self.chunk_biomes(chunk.0, chunk.1);
                trace::capture(|| self.structures.place(&self.density, &mut shape, chunk, neighbours, &biomes, &features))
            };
            state.borrow_mut().insert(chunk, shape);
            let mut label: Option<(std::rc::Rc<str>, Arc<str>)> = None;
            for (at, block, by) in traced {
                if label.as_ref().is_none_or(|(rc, _)| !std::rc::Rc::ptr_eq(rc, &by)) {
                    label = Some((by.clone(), Arc::from(&*by)));
                }
                writes.push(ReplayWrite { rank, chunk, at, block, label: label.as_ref().unwrap().1.clone() });
            }
            rank += 1;
            let Some(placed) = placed else { continue };
            let mut widen = |target: (i32, i32), (lo, hi): (i32, i32)| {
                let heights = touched.entry(target).or_insert((i32::MAX, i32::MIN));
                *heights = (heights.0.min(lo), heights.1.max(hi));
            };
            widen(chunk, placed.touched);
            spawners.extend(placed.spawners);
            spilled.insert(chunk, placed.spilled);
            for ([x, y, z], block, _) in placed.unclipped {
                let target = (x >> 4, z >> 4);
                if let Some(shape) = state.borrow_mut().get_mut(&target) {
                    shape.set_block((x & 15) as usize, y, (z & 15) as usize, block);
                    widen(target, (y, y));
                }
            }
        }
        let pieced = |(chunk, shape)| (chunk, Arc::new(Pieced::replayed(shape, touched.get(&chunk).copied(), spilled.remove(&chunk).unwrap_or_default())));
        let chunks = state.into_inner().into_iter().map(pieced).collect();
        Done { chunks, writes }
    }
}
