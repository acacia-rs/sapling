//! TODO: port `AcaciaTrunk` (featurelab features/tree.go).

use super::spec::{AcaciaTrunk, Tree};
use crate::feature::place::Ctx;

impl AcaciaTrunk {
    pub fn place(&self, _tree: &Tree, _ctx: &mut Ctx, _origin: [i32; 3]) -> Option<[i32; 3]> {
        None
    }
}
