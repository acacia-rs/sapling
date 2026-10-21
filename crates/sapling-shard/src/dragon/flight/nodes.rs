//! The dragon's 24 path nodes and its A* over them (`findClosestNode` 145997df0, `findPath`
//! 145999380; research/dragon-ai-decompile.md 1), Java's graph with BDS's truncation and cap:
//! twelve on a ring of 60, eight of 40 and four of 20 round the island, above its ground. With no
//! crystals left the search keeps off the outer ring.

use sapling_world::World as Level;

pub(super) const COUNT: usize = 24;
const MASKS: [u32; COUNT] = [
    0x1802, 0x2005, 0x200a, 0x4014, 0x8028, 0x8050, 0x100a0, 0x20140, 0x20280, 0x40500, 0x80a00, 0x80401, 0x182001, 0x305006, 0x20a008, 0x614030, 0x428040, 0xc50180, 0x8a0200, 0x941c00, 0xe83000, 0xd0e000, 0xb38000,
    0x7e0000,
];
/// Nodes are built once the columns this far round the middle are loaded.
pub(super) const BUILT_WITHIN: i32 = 60;
const SEA: i32 = 63;
const ABOVE_SEA: i32 = 10;
const CAP: i32 = 128;
/// No node this far (squared) from a point is its closest.
const FARTHEST: f32 = 10_000.0;
/// The first node a search uses with no crystals: the inner rings'.
const INNER: usize = 12;

pub(super) type Node = [i32; 3];

/// The heightmap's y at a column: the first cell over its highest block.
fn ground(level: &Level, x: i32, z: i32) -> i32 {
    level.surface(x, z).map_or(0, |y| y + 1)
}

/// The nodes over `level`'s island, in BDS's order.
pub(super) fn build(level: &Level) -> Vec<Node> {
    (0..COUNT)
        .map(|i| {
            let (k, step, radius, lift) = match i {
                0..12 => (i, 0.261_799_4_f32, 60.0, 5),
                12..20 => (i - 12, 0.392_699_1, 40.0, 15),
                _ => (i - 20, 0.785_398_2, 20.0, 5),
            };
            let angle = (k as f32 * step - std::f32::consts::PI) * 2.0;
            let (x, z) = ((angle.cos() * radius) as i32, (angle.sin() * radius) as i32);
            let y = (ground(level, x, z) + lift).max(SEA + ABOVE_SEA).min(CAP);
            [x, y, z]
        })
        .collect()
}

fn first(crystals: u32) -> usize {
    if crystals != 0 { 0 } else { INNER }
}

fn distance(a: Node, b: Node) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]) as f32).map(|d| d * d).sum::<f32>().sqrt()
}

/// The node nearest `at` of those a fight with `crystals` uses; 0 when none is near.
pub(super) fn closest(nodes: &[Node], crystals: u32, at: [f32; 3]) -> usize {
    let p = at.map(|c| c.floor() as i32);
    let mut best = (FARTHEST, 0);
    for (i, node) in nodes.iter().enumerate().skip(first(crystals)) {
        let d = (0..3).map(|axis| (node[axis] - p[axis]) as f32).map(|d| d * d).sum::<f32>();
        if d < best.0 {
            best = (d, i);
        }
    }
    best.1
}

#[derive(Clone, Copy)]
struct Point {
    total: f32,
    to_end: f32,
    score: f32,
    previous: Option<usize>,
    heap: Option<usize>,
    visited: bool,
}

/// Java's path heap on `score`, holding node indices.
struct Heap(Vec<usize>);

impl Heap {
    fn up(&mut self, points: &mut [Point], mut i: usize) {
        let node = self.0[i];
        let score = points[node].score;
        while i > 0 {
            let parent = (i - 1) >> 1;
            let above = self.0[parent];
            if score >= points[above].score {
                break;
            }
            self.0[i] = above;
            points[above].heap = Some(i);
            i = parent;
        }
        self.0[i] = node;
        points[node].heap = Some(i);
    }

    fn down(&mut self, points: &mut [Point], mut i: usize) {
        let node = self.0[i];
        let score = points[node].score;
        loop {
            let (left, right) = (1 + (i << 1), 2 + (i << 1));
            if left >= self.0.len() {
                break;
            }
            let left_score = points[self.0[left]].score;
            let right_score = self.0.get(right).map_or(f32::INFINITY, |&r| points[r].score);
            let child = if left_score < right_score { left } else { right };
            if left_score.min(right_score) >= score {
                break;
            }
            let below = self.0[child];
            self.0[i] = below;
            points[below].heap = Some(i);
            i = child;
        }
        self.0[i] = node;
        points[node].heap = Some(i);
    }

    fn push(&mut self, points: &mut [Point], node: usize) {
        self.0.push(node);
        let at = self.0.len() - 1;
        self.up(points, at);
    }

    fn pop(&mut self, points: &mut [Point]) -> Option<usize> {
        let top = *self.0.first()?;
        let last = self.0.pop()?;
        if !self.0.is_empty() {
            self.0[0] = last;
            self.down(points, 0);
        }
        points[top].heap = None;
        Some(top)
    }

    fn rescore(&mut self, points: &mut [Point], node: usize, score: f32) {
        let (old, at) = (points[node].score, points[node].heap.expect("in the heap"));
        points[node].score = score;
        if score < old { self.up(points, at) } else { self.down(points, at) }
    }
}

/// The nodes from `start` to `end` (or as near it as the graph goes), then `then`; `None` when
/// no step leaves `start`.
pub(super) fn path(nodes: &[Node], crystals: u32, start: usize, end: usize, then: Option<Node>) -> Option<Vec<Node>> {
    if nodes.len() < COUNT {
        return None;
    }
    let mut points = [Point { total: 0.0, to_end: 0.0, score: 0.0, previous: None, heap: None, visited: false }; COUNT];
    let to_end = distance(nodes[start], nodes[end]);
    (points[start].to_end, points[start].score) = (to_end, to_end);
    let mut heap = Heap(Vec::new());
    heap.push(&mut points, start);
    let mut nearest = start;
    let mut reached = None;
    while let Some(current) = heap.pop(&mut points) {
        if nodes[current] == nodes[end] {
            reached = Some(current);
            break;
        }
        if distance(nodes[current], nodes[end]) < distance(nodes[nearest], nodes[end]) {
            nearest = current;
        }
        points[current].visited = true;
        for next in first(crystals)..COUNT {
            if MASKS[current] & (1 << next) == 0 || points[next].visited {
                continue;
            }
            let total = points[current].total + distance(nodes[current], nodes[next]);
            let queued = points[next].heap.is_some();
            if queued && total >= points[next].total {
                continue;
            }
            let to_end = distance(nodes[next], nodes[end]);
            (points[next].previous, points[next].total, points[next].to_end) = (Some(current), total, to_end);
            if queued {
                heap.rescore(&mut points, next, total + to_end);
            } else {
                points[next].score = total + to_end;
                heap.push(&mut points, next);
            }
        }
    }
    let last = match reached {
        Some(end) => end,
        None if nearest == start => return None,
        None => nearest,
    };
    let mut walked = vec![nodes[last]];
    let mut at = last;
    while let Some(previous) = points[at].previous {
        walked.push(nodes[previous]);
        at = previous;
    }
    walked.reverse();
    walked.extend(then);
    Some(walked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rings_are_javas_and_without_crystals_a_path_keeps_to_the_inner_ones() {
        let nodes = build(&Level::void_in(sapling_world::END));
        assert_eq!([nodes[0], nodes[12], nodes[20]], [[60, 73, 0], [40, 73, 0], [20, 73, 0]]);
        assert_eq!(closest(&nodes, 0, [61.0, 80.0, 1.0]), 12, "the outer ring is left out");
        assert_eq!(closest(&nodes, 10, [61.0, 80.0, 1.0]), 0);
        let path = path(&nodes, 0, 12, 16, Some([0, 66, 0])).expect("a path");
        assert_eq!((path.first(), path.last()), (Some(&nodes[12]), Some(&[0, 66, 0])));
        assert!(path[..path.len() - 1].iter().all(|node| nodes[INNER..].contains(node)), "{path:?}");
    }
}
