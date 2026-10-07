//! TODO: port `mega_trunk` (with its branches, decoration and `base_cluster`) and the
//! `mega_canopy` / `mega_pine_canopy` shapes (featurelab features/tree.go).

use super::Anchor;
use super::spec::{Canopy, MegaTrunk, Tree};
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::random::Mt19937;

impl MegaTrunk {
    pub fn place(&self, _tree: &Tree, _ctx: &mut Ctx, _origin: [i32; 3]) -> Option<[i32; 3]> {
        None
    }
}

/// `Canopy::Mega` and `Canopy::MegaPine`.
pub fn canopy(_canopy: &Canopy, _tree: &Tree, _view: &mut View, _random: &mut Mt19937, _anchor: &Anchor) {}
