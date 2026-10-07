//! TODO: port `MangroveTrunk` (featurelab features/tree.go).

use super::spec::{MangroveTrunk, Tree};
use crate::feature::place::Ctx;

impl MangroveTrunk {
    pub fn place(&self, _tree: &Tree, _ctx: &mut Ctx, _origin: [i32; 3]) -> Option<[i32; 3]> {
        None
    }
}
