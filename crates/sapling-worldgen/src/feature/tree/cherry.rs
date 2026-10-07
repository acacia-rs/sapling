//! TODO: port `cherry_trunk` and `cherry_canopy` (featurelab features/tree.go).

use super::Anchor;
use super::spec::{CherryCanopy, CherryTrunk, Tree};
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::random::Mt19937;

impl CherryTrunk {
    pub fn place(&self, _tree: &Tree, _ctx: &mut Ctx, _origin: [i32; 3]) -> Option<[i32; 3]> {
        None
    }
}

impl CherryCanopy {
    pub fn place(&self, _tree: &Tree, _view: &mut View, _random: &mut Mt19937, _anchor: &Anchor) {}
}
