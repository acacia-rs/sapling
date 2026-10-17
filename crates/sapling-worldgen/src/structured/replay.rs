//! The structure stage in one logged BDS run's order (scripts/dbg_loads.py `--structure-order`):
//! the same engine with that run's post-process order and start map in place of the canonical
//! ones: docs/research/worldgen-bds-order.md, "Replaying a run".

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::order::{Logged, Order};
use super::{Stage, square};
use crate::block::Block;
use crate::cache::SourceCache;
use crate::structure::trace;
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

impl OverworldGenerator {
    /// Runs the structure stage as one BDS run did. Without `Load` steps mineshaft starts keep the
    /// canonical start map.
    #[doc(hidden)]
    pub fn set_structure_order(&mut self, steps: &[StructureStep]) {
        let logged = Logged::new(steps);
        // A dropped post-process would rerun through every one before it: the caches hold the whole run.
        let chunks = logged.order().len() * 9 + 256;
        self.carved = SourceCache::new(chunks * 2);
        self.generated_tops = SourceCache::new(chunks * 2);
        self.pieced = SourceCache::new(chunks * 2);
        self.staged = SourceCache::new(chunks * 20);
        self.structure_order = Order::Logged(logged);
    }

    /// Under a logged order, runs every post-process in that order once, so that none recurses
    /// through all those before it.
    pub(super) fn warm_logged(&self) {
        let Order::Logged(logged) = &self.structure_order else { return };
        logged.warm.call_once(|| {
            let order = logged.order();
            let wanted: Vec<(i32, i32)> = order.iter().flat_map(|&chunk| square(chunk, 1)).collect::<HashSet<_>>().into_iter().collect();
            self.carve_all(&wanted);
            order.iter().for_each(|&chunk| drop(self.source(chunk)));
        });
    }

    /// The carved terrain of `chunks`, generated on every core.
    fn carve_all(&self, chunks: &[(i32, i32)]) {
        let next = AtomicUsize::new(0);
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| {
                    while let Some(&(x, z)) = chunks.get(next.fetch_add(1, Ordering::Relaxed)) {
                        self.carved(x, z);
                    }
                });
            }
        });
    }

    /// Every write of the replayed structure stage, in the run's order; empty without `set_structure_order`.
    #[doc(hidden)]
    pub fn structure_replay_writes(&self) -> &[ReplayWrite] {
        let Order::Logged(logged) = &self.structure_order else { return &[] };
        self.warm_structures();
        logged.writes.get_or_init(|| {
            let mut writes = Vec::new();
            for (rank, chunk) in logged.order().into_iter().enumerate() {
                let (_, traced) = trace::capture(|| self.post_process(chunk));
                let mut label: Option<(std::rc::Rc<str>, Arc<str>)> = None;
                for (at, block, by) in traced {
                    if label.as_ref().is_none_or(|(rc, _)| !std::rc::Rc::ptr_eq(rc, &by)) {
                        label = Some((by.clone(), Arc::from(&*by)));
                    }
                    writes.push(ReplayWrite { rank, chunk, at, block, label: label.as_ref().unwrap().1.clone() });
                }
            }
            writes
        })
    }

    /// The feature trace of the replayed structure stage (pieces' features: `element` lines and their
    /// writes); empty on a second call.
    #[doc(hidden)]
    pub fn structure_replay_trace(&self) -> Vec<String> {
        feature::trace::capture(|| self.structure_replay_writes().len()).1
    }
}
