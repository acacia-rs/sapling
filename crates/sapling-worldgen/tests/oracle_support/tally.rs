//! Counting verdicts and printing them per stage.

use std::collections::BTreeMap;

use sapling_worldgen::Block;

use super::{BdsCell, Stage, Verdict, judge};

type At = (i32, i32, i32);

#[derive(Default)]
struct Pairs(BTreeMap<String, (u64, At)>);

impl Pairs {
    fn add(&mut self, label: String, at: At) {
        self.0.entry(label).or_insert((0, at)).0 += 1;
    }

    fn print(&self, indent: &str, limit: usize) {
        let mut pairs: Vec<_> = self.0.iter().collect();
        pairs.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        for (label, (count, at)) in pairs.iter().take(limit) {
            println!("{indent}{count:>9}  {label}  first {at:?}");
        }
    }
}

fn label(ours: Block, bds: &BdsCell) -> String {
    let liquid = bds.liquid.as_deref().map_or(String::new(), |l| format!(" +{l}"));
    format!("ours {} bds {}[{}]{liquid}", ours.name(), bds.block, bds.states)
}

#[derive(Default)]
pub struct Tally {
    unstable: u64,
    matched: BTreeMap<Stage, u64>,
    errors: BTreeMap<Stage, Pairs>,
    later: BTreeMap<&'static str, Pairs>,
}

impl Tally {
    pub fn add(&mut self, ours: Block, bds: &BdsCell, stable: bool, at: At) {
        if !stable {
            self.unstable += 1;
            return;
        }
        match judge(ours, bds, at.1) {
            Verdict::Match(stage) => *self.matched.entry(stage).or_default() += 1,
            Verdict::Error(stage) => self.errors.entry(stage).or_default().add(label(ours, bds), at),
            Verdict::Later(why) => self.later.entry(why).or_default().add(label(ours, bds), at),
        }
    }

    pub fn print(&self) {
        println!("  {} cells unstable between BDS runs", self.unstable);
        for stage in [Stage::Floor, Stage::Deepslate, Stage::Veins, Stage::Surface, Stage::Shape] {
            let matched = self.matched.get(&stage).copied().unwrap_or(0);
            let errors = self.errors.get(&stage);
            let wrong: u64 = errors.map_or(0, |p| p.0.values().map(|v| v.0).sum());
            let total = matched + wrong;
            println!("  {stage:?}: {total} cells, {:.4}% agree, {wrong} differ", 100.0 * matched as f64 / total.max(1) as f64);
            if let Some(errors) = errors {
                errors.print("    ", 8);
            }
        }
        for (why, pairs) in &self.later {
            let count: u64 = pairs.0.values().map(|v| v.0).sum();
            println!("  later stages, {why}: {count}");
            pairs.print("    ", 4);
        }
    }
}

/// The highest solid block of each column, ours against BDS's block at the same y.
#[derive(Default)]
pub struct TopTally {
    matched: u64,
    later: u64,
    errors: Pairs,
}

impl TopTally {
    pub fn add(&mut self, ours: Block, bds: &BdsCell, at: At) {
        match judge(ours, bds, at.1) {
            Verdict::Match(_) => self.matched += 1,
            Verdict::Later(_) => self.later += 1,
            Verdict::Error(_) => self.errors.add(label(ours, bds), at),
        }
    }

    pub fn print(&self) {
        let wrong: u64 = self.errors.0.values().map(|v| v.0).sum();
        let total = self.matched + wrong;
        println!(
            "  surface top block: {total} columns, {:.4}% agree, {wrong} differ, {} covered by later stages",
            100.0 * self.matched as f64 / total.max(1) as f64,
            self.later
        );
        self.errors.print("    ", 10);
    }
}

/// Per-block biome agreement, with the most common (ours, BDS) disagreements.
#[derive(Default)]
pub struct BiomeTally {
    matched: u64,
    pairs: BTreeMap<(u32, Option<u32>), (u64, At)>,
}

impl BiomeTally {
    pub fn add(&mut self, ours: u32, bds: Option<u32>, at: At) {
        if Some(ours) == bds {
            self.matched += 1;
        } else {
            self.pairs.entry((ours, bds)).or_insert((0, at)).0 += 1;
        }
    }

    pub fn print(&self) {
        let mismatched: u64 = self.pairs.values().map(|p| p.0).sum();
        let total = self.matched + mismatched;
        println!("  biomes: {total} blocks, {:.4}% agree, {mismatched} differ", 100.0 * self.matched as f64 / total.max(1) as f64);
        let mut pairs: Vec<_> = self.pairs.iter().collect();
        pairs.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        for ((ours, bds), (count, at)) in pairs.iter().take(12) {
            println!("    ours {ours:>3} bds {bds:?} {count:>9}  first {at:?}");
        }
    }
}
