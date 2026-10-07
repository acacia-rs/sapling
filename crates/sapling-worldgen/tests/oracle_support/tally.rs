//! Counting verdicts and printing them per stage.

use std::collections::BTreeMap;

use sapling_worldgen::Block;

use super::BdsCell;
use super::judge::{Context, Stage, Verdict, judge};

type At = (i32, i32, i32);

/// Per label: the count and a few sample positions (the 1st, 10th, 100th, ... occurrence).
#[derive(Default)]
struct Pairs(BTreeMap<String, (u64, Vec<At>)>);

impl Pairs {
    fn add(&mut self, label: String, at: At) {
        let entry = self.0.entry(label).or_default();
        entry.0 += 1;
        if [1, 10, 100, 1000, 10000].contains(&entry.0) {
            entry.1.push(at);
        }
    }

    fn count(&self) -> u64 {
        self.0.values().map(|v| v.0).sum()
    }

    fn print(&self, indent: &str, limit: usize) {
        let mut pairs: Vec<_> = self.0.iter().collect();
        pairs.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        for (label, (count, at)) in pairs.iter().take(limit) {
            println!("{indent}{count:>9}  {label}  at {at:?}");
        }
    }
}

fn label(ours: Block, bds: &BdsCell) -> String {
    let liquid = bds.liquid.as_deref().map_or(String::new(), |l| format!(" +{l}"));
    format!("ours {} bds {}[{}]{liquid}", ours.name(), bds.block, bds.states)
}

fn percent(n: u64, of: u64) -> f64 {
    100.0 * n as f64 / of.max(1) as f64
}

#[derive(Default)]
pub struct Tally {
    unstable: u64,
    matched: BTreeMap<Stage, u64>,
    errors: BTreeMap<Stage, Pairs>,
    later: BTreeMap<&'static str, Pairs>,
}

impl Tally {
    pub fn add(&mut self, ours: Block, bds: &BdsCell, stable: bool, at: At, context: Context) {
        if !stable {
            self.unstable += 1;
            return;
        }
        match judge(ours, bds, at.1, context) {
            Verdict::Match(stage) => *self.matched.entry(stage).or_default() += 1,
            Verdict::Error(stage) => self.errors.entry(stage).or_default().add(label(ours, bds), at),
            Verdict::Later(why) => self.later.entry(why).or_default().add(label(ours, bds), at),
        }
    }

    pub fn print(&self) {
        let matched: u64 = self.matched.values().sum();
        let wrong: u64 = self.errors.values().map(Pairs::count).sum();
        let later: u64 = self.later.values().map(Pairs::count).sum();
        let stable = matched + wrong + later;
        println!(
            "  exact blocks: {stable} stable cells, {:.4}% identical, {wrong} wrong, {later} left to later stages; {} unstable between BDS runs",
            percent(matched, stable),
            self.unstable
        );
        for stage in [Stage::Floor, Stage::Deepslate, Stage::Veins, Stage::Surface, Stage::Carvers, Stage::Lakes, Stage::Shape] {
            let matched = self.matched.get(&stage).copied().unwrap_or(0);
            let errors = self.errors.get(&stage);
            let wrong = errors.map_or(0, Pairs::count);
            let total = matched + wrong;
            println!("  {stage:?}: {total} cells, {:.4}% agree, {wrong} differ", percent(matched, total));
            if let Some(errors) = errors {
                errors.print("    ", 8);
            }
        }
        for (why, pairs) in &self.later {
            println!("  later stages, {why}: {}", pairs.count());
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
    pub fn add(&mut self, ours: Block, bds: &BdsCell, at: At, context: Context) {
        match judge(ours, bds, at.1, context) {
            Verdict::Match(_) => self.matched += 1,
            Verdict::Later(_) => self.later += 1,
            Verdict::Error(_) => self.errors.add(label(ours, bds), at),
        }
    }

    pub fn print(&self) {
        let wrong = self.errors.count();
        let total = self.matched + wrong;
        println!(
            "  surface top block: {total} columns, {:.4}% agree, {wrong} differ, {} covered by later stages",
            percent(self.matched, total),
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
        println!("  biomes: {total} blocks, {:.4}% agree, {mismatched} differ", percent(self.matched, total));
        let mut pairs: Vec<_> = self.pairs.iter().collect();
        pairs.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        for ((ours, bds), (count, at)) in pairs.iter().take(12) {
            println!("    ours {ours:>3} bds {bds:?} {count:>9}  first {at:?}");
        }
    }
}
