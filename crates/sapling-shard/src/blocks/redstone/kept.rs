//! Comparators set again by command in their own block type: BDS keeps the circuit's component, so
//! the comparator gives out what it gave until one of its inputs changes, whatever mode its new state
//! names (research/captures/bds-e8cmpset.txt; docs/research/redstone.md, "Parity drill").

use std::collections::{HashMap, HashSet};

use sapling_api::BlockPos;

use super::capacitors::compared;

#[derive(Default)]
pub(super) struct Kept {
    /// What each comparator read behind and at its side at its last evaluation.
    inputs: HashMap<BlockPos, (u8, u8)>,
    /// Comparators whose output stands until those inputs change.
    held: HashSet<BlockPos>,
}

impl Kept {
    pub(super) fn hold(&mut self, pos: BlockPos) {
        if self.inputs.contains_key(&pos) {
            self.held.insert(pos);
        }
    }

    /// The output of the comparator at `pos`, which gave `output` before.
    pub(super) fn next(&mut self, pos: BlockPos, behind: u8, side: u8, subtracts: bool, output: u8) -> u8 {
        let unchanged = self.inputs.insert(pos, (behind, side)) == Some((behind, side));
        if unchanged && self.held.contains(&pos) {
            return output;
        }
        self.held.remove(&pos);
        compared(behind, side, subtracts)
    }

    pub(super) fn retain(&mut self, keep: impl Fn(&BlockPos) -> bool) {
        self.inputs.retain(|pos, _| keep(pos));
        self.held.retain(|pos| keep(pos));
    }
}
