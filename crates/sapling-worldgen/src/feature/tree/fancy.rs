//! TODO: port `FancyTrunk` (featurelab features/tree.go).

use super::spec::{FancyTrunk, Tree};
use crate::feature::place::Ctx;

impl FancyTrunk {
    pub fn place(&self, _tree: &Tree, _ctx: &mut Ctx, _origin: [i32; 3]) -> Option<[i32; 3]> {
        None
    }
}
