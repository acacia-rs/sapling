//! `minecraft:tree_feature` (`TreeFeature`): a trunk shape, then its canopy at the anchors the
//! trunk returns. Ported from stirante/featurelab `features/tree.go` (MIT, (c) 2026 stirante),
//! checked against BDS oracle worlds.

mod acacia;
mod canopy;
mod cherry;
mod common;
mod fallen;
mod fancy;
mod mangrove;
mod mangrove_canopy;
mod mangrove_roots;
mod mega;
mod mega_canopy;
mod roofed;
mod simple;
pub mod spec;

use super::place::Ctx;
use super::view::View;
use crate::random::Mt19937;
use spec::{Canopy, Trunk, Tree};

/// A canopy call: where, and the logs it may grow from (the random-spread canopy reads them).
pub struct Anchor {
    pub at: [i32; 3],
    pub candidates: Vec<[i32; 3]>,
}

impl Anchor {
    pub fn at(at: [i32; 3]) -> Self {
        Self { at, candidates: Vec::new() }
    }
}

impl Tree {
    /// Where the tree stood, if it did.
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        match &self.trunk {
            Trunk::Simple(trunk) => trunk.place(self, ctx, at),
            Trunk::Acacia(trunk) => trunk.place(self, ctx, at),
            Trunk::Fancy(trunk) => trunk.place(self, ctx, at),
            Trunk::Mega(trunk) => trunk.place(self, ctx, at),
            Trunk::Mangrove(trunk) => trunk.place(self, ctx, at),
            Trunk::Cherry(trunk) => trunk.place(self, ctx, at),
            Trunk::Fallen(trunk) => trunk.place(self, ctx, at),
        }
    }

    /// `mangrove_roots`, between the height draw and the trunk: the trunk's new origin, or None
    /// when the roots fail (the whole tree fails).
    pub fn roots_then(&self, ctx: &mut Ctx, origin: [i32; 3], height: i32) -> Option<[i32; 3]> {
        match self.roots {
            None => Some(origin),
            Some(roots) => roots.place(self, ctx, origin, height),
        }
    }

    pub fn place_canopies(&self, ctx: &mut Ctx, anchors: &[Anchor]) {
        if let Some(canopy) = self.canopy {
            for anchor in anchors {
                canopy.place(self, ctx.view, ctx.random, anchor);
            }
        }
    }
}

impl Canopy {
    pub fn place(&self, tree: &Tree, view: &mut View, random: &mut Mt19937, anchor: &Anchor) {
        match self {
            Canopy::Mega { .. } | Canopy::MegaPine { .. } => mega::canopy(self, tree, view, random, anchor),
            Canopy::Roofed { .. } => roofed::canopy(self, tree, view, random, anchor),
            Canopy::Mangrove(canopy) => canopy.place(tree, view, random, anchor),
            Canopy::Cherry(canopy) => canopy.place(tree, view, random, anchor),
            // TODO: random_spread_canopy (azalea trees of lush caves).
            Canopy::RandomSpread { .. } => {}
            _ => canopy::place(self, tree, view, random, anchor),
        }
    }
}
