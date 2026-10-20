//! When a chunk's structure post-process runs relative to the others, and what its mineshaft
//! starts' order is: our canonical order, or one BDS run's logged one
//! (docs/research/worldgen-bds-order.md, "One engine").

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Once, OnceLock};

use super::replay::{ReplayWrite, StructureStep};
use crate::OverworldGenerator;
use crate::structure::{StartMap, StructureKind};

/// A post-process's place in the order: sources ranked lower ran before it.
pub type Rank = u32;

type Starts = Arc<[(i32, i32)]>;

/// One BDS run's structure stage, as logged.
pub struct Logged {
    pub(super) steps: Vec<StructureStep>,
    ranks: HashMap<(i32, i32), Rank>,
    /// Per rank, the start map's list when that chunk was post-processed; empty without logged loads.
    shaft_orders: OnceLock<Vec<Starts>>,
    /// Set once every post-process has run in the run's order (replay.rs).
    pub(super) warm: Once,
    pub(super) writes: OnceLock<Vec<ReplayWrite>>,
}

#[derive(Default)]
pub enum Order {
    /// Nine phases by the chunk's position modulo 3: chunks of one phase are three apart, so no
    /// two of them read or write a common chunk and any sequence of the phases' chunks is one run.
    #[default]
    Canonical,
    Logged(Logged),
}

impl Order {
    pub fn is_canonical(&self) -> bool {
        matches!(self, Order::Canonical)
    }

    /// None: a chunk the logged run did not post-process.
    pub fn rank(&self, chunk: (i32, i32)) -> Option<Rank> {
        match self {
            Order::Canonical => Some((chunk.0.rem_euclid(3) * 3 + chunk.1.rem_euclid(3)) as Rank),
            Order::Logged(logged) => logged.ranks.get(&chunk).copied(),
        }
    }
}

impl Logged {
    /// A chunk's second `Post` is dropped.
    pub fn new(steps: &[StructureStep]) -> Self {
        let mut seen = HashSet::new();
        let once = |step: &&StructureStep| !matches!(**step, StructureStep::Post(chunk) if !seen.insert(chunk));
        let steps: Vec<StructureStep> = steps.iter().filter(once).copied().collect();
        let ranks = Self::posts(&steps).enumerate().map(|(rank, chunk)| (chunk, rank as Rank)).collect();
        Self { steps, ranks, shaft_orders: OnceLock::new(), warm: Once::new(), writes: OnceLock::new() }
    }

    fn posts(steps: &[StructureStep]) -> impl Iterator<Item = (i32, i32)> + '_ {
        steps.iter().filter_map(|step| if let StructureStep::Post(chunk) = step { Some(*chunk) } else { None })
    }

    /// The post-processed chunks in the run's order.
    pub fn order(&self) -> Vec<(i32, i32)> {
        Self::posts(&self.steps).collect()
    }

    fn shaft_orders(&self, is_start: &dyn Fn((i32, i32)) -> bool) -> Vec<Starts> {
        let mut starts = StartMap::default();
        let mut current: Option<Starts> = None;
        let mut orders = Vec::new();
        if !self.steps.iter().any(|step| matches!(step, StructureStep::Load(_))) {
            return orders;
        }
        for step in &self.steps {
            match *step {
                StructureStep::Load(chunk) => {
                    starts.load(chunk, is_start);
                    current = None;
                }
                StructureStep::Post(_) => orders.push(current.get_or_insert_with(|| starts.order().into()).clone()),
            }
        }
        orders
    }
}

impl OverworldGenerator {
    /// `starts`, the mineshafts reaching `chunk`, in the order the start map lists them while the chunk
    /// is post-processed: the logged run's, else descending x then z.
    pub(super) fn shaft_order(&self, chunk: (i32, i32), starts: &mut [(i32, i32)]) {
        let is_start = |start| self.structures.is_start(&self.density, StructureKind::Mineshaft, start);
        if let Order::Logged(logged) = &self.structure_order {
            let orders = logged.shaft_orders.get_or_init(|| logged.shaft_orders(&is_start));
            if let Some(order) = logged.ranks.get(&chunk).and_then(|&rank| orders.get(rank as usize)) {
                return starts.sort_by_key(|start| order.iter().position(|listed| listed == start));
            }
        }
        starts.sort_unstable_by(|a, b| b.cmp(a));
    }
}
