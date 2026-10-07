//! TODO: port `FallenTrunk` (featurelab features/tree.go).

use super::spec::{FallenTrunk, Tree};
use crate::feature::place::Ctx;

impl FallenTrunk {
    pub fn place(&self, _tree: &Tree, _ctx: &mut Ctx, _origin: [i32; 3]) -> Option<[i32; 3]> {
        None
    }
}
