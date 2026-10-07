//! `RTree::build` (148060510): Java's bucketing by the cheapest parameter, on BDS's sort.

use super::sort::sort;
use super::{Node, key};
use crate::biome::builder::{Range, Space};

/// The node's parameter midpoint, (min + max) / 2 truncated toward zero, or its magnitude.
fn midpoint(node: &Node, param: usize, abs: bool) -> i64 {
    let Range(min, max) = node.space[param];
    let mid = (min + max) / 2;
    if abs { mid.abs() } else { mid }
}

/// `RTree::Node::sort` (148060320): midpoints compared from `param` on, wrapping around all 7.
fn sort_by_param(nodes: &[Node], ids: &mut [usize], param: usize, abs: bool) {
    let less = |&a: &usize, &b: &usize| -> bool {
        for i in 0..7 {
            let p = (param + i) % 7;
            let (ka, kb) = (midpoint(&nodes[a], p, abs), midpoint(&nodes[b], p, abs));
            if ka != kb {
                return ka < kb;
            }
        }
        false
    };
    sort(ids, &less);
}

fn bounds(nodes: &[Node], children: &[usize]) -> Space {
    let mut space = [Range(i64::MAX, i64::MIN); 7];
    for &child in children {
        for (bound, range) in space.iter_mut().zip(nodes[child].space) {
            *bound = Range(bound.0.min(range.0), bound.1.max(range.1));
        }
    }
    space
}

fn subtree(nodes: &mut Vec<Node>, children: Vec<usize>) -> usize {
    let space = bounds(nodes, &children);
    nodes.push(Node { space, key: key(&space), children, biome: None });
    nodes.len() - 1
}

/// Java's `(int) Math.pow(6, Math.floor(Math.log(n - 0.01) / Math.log(6)))`, with BDS's f32
/// logarithm and floor and f64 power.
fn bucket_size(count: usize) -> usize {
    let exponent = ((count as f32 + -0.01).ln() / 1.791_759_5).floor();
    6f64.powf(exponent as f64) as usize
}

fn cost(space: &Space) -> i64 {
    space.iter().map(|r| (r.0 - r.1).abs()).sum()
}

pub fn build(nodes: &mut Vec<Node>, mut children: Vec<usize>) -> Option<usize> {
    match children.len() {
        0 => return None,
        1 => return Some(children[0]),
        2..=6 => {
            let less = |&a: &usize, &b: &usize| nodes[a].key < nodes[b].key;
            sort(&mut children, &less);
            return Some(subtree(nodes, children));
        }
        _ => {}
    }
    let size = bucket_size(children.len());
    let mut best: Option<(i64, usize, Vec<Vec<usize>>)> = None;
    for param in 0..7 {
        sort_by_param(nodes, &mut children, param, false);
        let buckets: Vec<Vec<usize>> = children.chunks(size).map(<[usize]>::to_vec).collect();
        let total: i64 = buckets.iter().map(|b| cost(&bounds(nodes, b))).sum();
        if best.as_ref().is_none_or(|(c, _, _)| total < *c) {
            best = Some((total, param, buckets));
        }
    }
    let (_, param, buckets) = best.expect("seven candidates");
    let mut bucket_ids: Vec<usize> = buckets.into_iter().map(|b| subtree(nodes, b)).collect();
    sort_by_param(nodes, &mut bucket_ids, param, true);
    let built: Vec<usize> = bucket_ids.into_iter().filter_map(|b| build(nodes, nodes[b].children.clone())).collect();
    Some(subtree(nodes, built))
}
