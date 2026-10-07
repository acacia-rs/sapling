//! TODO: port `mangrove_canopy` (featurelab features/tree.go).

use super::Anchor;
use super::spec::{MangroveCanopy, Tree};
use crate::feature::view::View;
use crate::random::Mt19937;

impl MangroveCanopy {
    pub fn place(&self, _tree: &Tree, _view: &mut View, _random: &mut Mt19937, _anchor: &Anchor) {}
}
