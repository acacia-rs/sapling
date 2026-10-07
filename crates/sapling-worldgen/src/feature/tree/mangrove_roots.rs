//! TODO: port `mangrove_roots` (featurelab features/tree.go).

use super::spec::{MangroveRoots, Tree};
use crate::feature::place::Ctx;

impl MangroveRoots {
    /// Runs between the trunk's height draw and the trunk: the trunk's new origin, or None when
    /// the roots fail (and with them the tree).
    pub fn place(&self, _tree: &Tree, _ctx: &mut Ctx, _origin: [i32; 3], _height: i32) -> Option<[i32; 3]> {
        None
    }
}
