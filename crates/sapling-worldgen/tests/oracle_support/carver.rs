//! The carver stage measured on its own: cells our terrain shape has solid, split by whether BDS
//! emptied them and whether our carvers did.

use std::collections::BTreeMap;

use sapling_worldgen::CellKind;

use super::BdsCell;

#[derive(Default)]
pub struct CarverTally {
    /// Both emptied it, to the same kind.
    matched: u64,
    /// Both emptied it, to different kinds (air against water, ...).
    wrong_fluid: u64,
    /// We carved, BDS kept it solid.
    extra: u64,
    /// BDS emptied it, we did not (carvers not ported yet, lakes, or misses).
    missed: u64,
    missed_by_band: BTreeMap<i32, u64>,
    extra_by_band: BTreeMap<i32, u64>,
    first_extra: Option<(i32, i32, i32)>,
    first_missed: Option<(i32, i32, i32)>,
}

impl CarverTally {
    /// `before` is our shape before carving, `after` after; unstable cells are skipped by the caller.
    pub fn add(&mut self, before: CellKind, after: CellKind, bds: &BdsCell, at: (i32, i32, i32)) {
        if before != CellKind::Solid {
            return;
        }
        let band = at.1.div_euclid(32) * 32;
        match (after != CellKind::Solid, bds.kind != CellKind::Solid) {
            (true, true) if after == bds.kind => self.matched += 1,
            (true, true) => self.wrong_fluid += 1,
            (true, false) => {
                self.extra += 1;
                *self.extra_by_band.entry(band).or_default() += 1;
                self.first_extra.get_or_insert(at);
            }
            (false, true) => {
                self.missed += 1;
                *self.missed_by_band.entry(band).or_default() += 1;
                self.first_missed.get_or_insert(at);
            }
            (false, false) => {}
        }
    }

    pub fn print(&self) {
        let carved = self.matched + self.wrong_fluid + self.extra;
        let emptied = self.matched + self.wrong_fluid + self.missed;
        let percent = |n: u64, of: u64| 100.0 * n as f64 / of.max(1) as f64;
        println!(
            "  carvers: {carved} cells carved, {emptied} emptied by BDS; precision {:.3}%, recall {:.3}% \
             (matched {}, wrong fluid {}, extra {}, missed {})",
            percent(self.matched, carved),
            percent(self.matched, emptied),
            self.matched,
            self.wrong_fluid,
            self.extra,
            self.missed
        );
        let bands = |m: &BTreeMap<i32, u64>| m.iter().map(|(y, n)| format!("{y}:{n}")).collect::<Vec<_>>().join(" ");
        println!("      extra by y/32 {}  first {:?}", bands(&self.extra_by_band), self.first_extra);
        println!("      missed by y/32 {}  first {:?}", bands(&self.missed_by_band), self.first_missed);
    }
}
