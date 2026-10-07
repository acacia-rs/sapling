//! BDS's biome search tree (`RTree::create` 1480610f0, build 148060510, `RTree::search`
//! 1480616f0): Java's `Climate.RTree` built over BDS's own entry list with MSVC's unstable sort,
//! so points equally far from several leaves get BDS's leaf.

mod build;
mod sort;

use super::builder::{Entry, Range, Space};

/// Quantized target: temperature, humidity, continentalness, erosion, depth, weirdness (offset 0).
pub type ClimatePoint = [i64; 6];

struct Node {
    space: Space,
    /// Sum of the ranges' midpoints, what nodes of fewer than 7 siblings are sorted by.
    key: i64,
    children: Vec<usize>,
    biome: Option<u32>,
}

/// A leaf of the tree; carry it to the next search of the same column as its warm start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leaf(usize);

pub struct RTree {
    nodes: Vec<Node>,
    root: usize,
}

fn key(space: &Space) -> i64 {
    space.iter().map(|r| (r.0 + r.1) / 2).sum()
}

/// Squared distance to the range box, offset included (always 0 against the target's 0).
fn distance(space: &Space, point: &ClimatePoint) -> i64 {
    let mut total = 0i64;
    for (i, &Range(min, max)) in space.iter().enumerate() {
        let value = point.get(i).copied().unwrap_or(0);
        let d = if value > max {
            value - max
        } else if value < min {
            min - value
        } else {
            0
        };
        total = total.wrapping_add(d.wrapping_mul(d));
    }
    total
}

impl RTree {
    pub fn new(entries: &[Entry]) -> Self {
        let mut nodes: Vec<Node> = entries.iter().map(|e| Node { space: e.space, key: key(&e.space), children: Vec::new(), biome: Some(e.biome) }).collect();
        let leaves: Vec<usize> = (0..nodes.len()).collect();
        let root = build::build(&mut nodes, leaves).expect("a non-empty biome list");
        Self { nodes, root }
    }

    pub fn biome(&self, leaf: Leaf) -> u32 {
        self.nodes[leaf.0].biome.expect("a leaf")
    }

    /// The nearest leaf; `warm` (the previous search's leaf) starts as the best candidate.
    pub fn nearest(&self, point: &ClimatePoint, warm: Option<Leaf>) -> Leaf {
        let mut best = match warm {
            Some(Leaf(i)) => (i, distance(&self.nodes[i].space, point)),
            None => (usize::MAX, i64::MAX),
        };
        self.search(self.root, point, &mut best);
        Leaf(best.0)
    }

    fn search(&self, node: usize, point: &ClimatePoint, best: &mut (usize, i64)) {
        for &child in &self.nodes[node].children {
            let d = distance(&self.nodes[child].space, point);
            if d < best.1 {
                if self.nodes[child].biome.is_some() {
                    *best = (child, d);
                } else {
                    self.search(child, point, best);
                }
            }
        }
    }
}
