//! `mangrove_canopy`: leaves jittered around every trunk and branch log, then vines and hanging
//! propagules under a shuffled selection of them.

use std::collections::HashSet;

use super::Anchor;
use super::common::{allows, pick_weighted};
use super::spec::{MangroveCanopy, Tree};
use crate::block::Block;
use crate::feature::view::View;
use crate::random::Mt19937;

/// Cells under a propagule that must be air before it hangs.
const SEARCH_DEPTH: i32 = 2;

impl MangroveCanopy {
    pub fn place(&self, tree: &Tree, view: &mut View, random: &mut Mt19937, anchor: &Anchor) {
        if anchor.candidates.is_empty() {
            return;
        }
        let height = self.height.draw_inclusive(random);
        let radius = self.radius.draw_inclusive(random);
        let mut leaves = Vec::new();
        for &[x, y, z] in &anchor.candidates {
            for _ in 0..self.attempts {
                let leaf = pick_weighted(self.leaves, random);
                let mut jitter = |bound: i32| {
                    let a = random.next_int(bound);
                    a - random.next_int(bound)
                };
                let dx = jitter(radius);
                let dy = jitter(height);
                let dz = jitter(radius);
                let p = [x + dx, y + dy, z + dz];
                // featurelab approximates BDS's material gate as may_replace or air.
                let existing = view.block(p[0], p[1], p[2]);
                if allows(tree.may_replace, existing) || existing == Block::Air {
                    view.set_block(p[0], p[1], p[2], leaf);
                    leaves.push(p);
                }
            }
        }
        for i in 1..leaves.len() {
            let j = random.next_int(i as i32 + 1) as usize;
            leaves.swap(i, j);
        }
        if let Some(decoration) = &self.decoration {
            for &p in &leaves {
                decoration.place(view, p, [true; 4], random);
            }
        }
        let mut taken = HashSet::new();
        for &[x, y, z] in &leaves {
            let below = [x, y - 1, z];
            if taken.contains(&below) || !self.hanging_chance.roll(random) {
                continue;
            }
            if (1..=SEARCH_DEPTH).any(|dy| view.block(x, y - dy, z) != Block::Air) {
                continue;
            }
            view.set_block(x, y - 1, z, self.hanging);
            for dx in -1..=1 {
                for dz in -1..=1 {
                    taken.insert([x + dx, y - 1, z + dz]);
                }
            }
        }
    }
}
