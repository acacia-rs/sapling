//! The canopy shapes without a trunk coupling: simple (`canopy`), acacia, pine, spruce and fancy.
//! The others live beside their trunks.

use super::Anchor;
use super::common::{allows, leaf_replaceable};
use super::spec::{Canopy, SimpleCanopy, Tree};
use crate::block::Block;
use crate::feature::view::View;
use crate::random::Mt19937;
use crate::terrain::MIN_Y;

pub fn place(canopy: &Canopy, tree: &Tree, view: &mut View, random: &mut Mt19937, anchor: &Anchor) {
    let at = anchor.at;
    match canopy {
        Canopy::Simple(simple) => simple.place(view, at, random),
        &Canopy::Acacia { leaf, size, simplify } => acacia(view, at, leaf, size, simplify, tree.may_replace),
        &Canopy::Pine { leaf, height, base_radius } => {
            let height = height.draw(random);
            let top_cap = random.next_int(height + 1) + base_radius;
            pine(view, at, leaf, height, top_cap, tree.may_replace);
        }
        &Canopy::Spruce { leaf, lower_offset, upper_offset, max_radius } => {
            spruce(view, at, random, leaf, (lower_offset, upper_offset, max_radius), tree.may_grow_on)
        }
        &Canopy::Fancy { leaf, height, radius } => fancy(view, at, leaf, height, radius, tree.may_replace),
        _ => {}
    }
}

/// `random_spread_canopy` (azalea trees, 1438f57d0): height then radius drawn as ranges (a draw even
/// when constant), then `attempts` leaves around every log the trunk offered, each the weighted leaf
/// then per axis `nextInt(r) - r + nextInt(r) + 1` (x, y, z).
pub fn random_spread(view: &mut View, random: &mut Mt19937, logs: &[[i32; 3]], leaves: &[(Block, i32)], [height, radius]: [i32; 2], attempts: i32, may_replace: &[Block]) {
    let height = super::spec::Range(height, height).draw_inclusive_always(random);
    let radius = super::spec::Range(radius, radius).draw_inclusive_always(random);
    for &[x, y, z] in logs {
        for _ in 0..attempts {
            let leaf = super::common::pick_weighted(leaves, random);
            let mut jitter = |bound: i32| {
                let a = random.next_int(bound) - bound;
                a + random.next_int(bound) + 1
            };
            let (dx, dy, dz) = (jitter(radius), jitter(height), jitter(radius));
            if super::common::valid_tree_position(view, [x + dx, y + dy, z + dz], may_replace) {
                view.set_block(x + dx, y + dy, z + dz, leaf);
            }
        }
    }
}

/// Writes `leaf` where the cell passes `gate`.
fn put(view: &mut View, [x, y, z]: [i32; 3], leaf: Block, gate: impl Fn(Block) -> bool) {
    if gate(view.block(x, y, z)) {
        view.set_block(x, y, z, leaf);
    }
}

/// A square of `radius` around `(x, z)`, its four corners left out when `cut` (and `radius > 0`
/// for the spruce and pine rings).
fn ring(view: &mut View, [x, y, z]: [i32; 3], radius: i32, cut_at_zero: bool, leaf: Block, gate: &impl Fn(Block) -> bool) {
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            if (cut_at_zero || radius != 0) && dx.abs() == radius && dz.abs() == radius {
                continue;
            }
            put(view, [x + dx, y, z + dz], leaf, gate);
        }
    }
}

impl SimpleCanopy {
    /// `run * dy` in int, to float, times the stored `1 / rise`, truncated.
    fn slope(&self, dy: i32) -> i32 {
        let inverse = if self.rise != 0 { 1.0 / self.rise as f32 } else { 0.0 };
        ((self.run * dy) as f32 * inverse) as i32
    }

    /// Past the end of a short `variation_chance` BDS reads on; the last entry stands in (featurelab).
    fn cut_corner(&self, layer: i32, random: &mut Mt19937) -> bool {
        let Some(last) = self.variation_chance.len().checked_sub(1) else { return false };
        self.variation_chance[(layer as usize).min(last)].roll(random)
    }

    fn place(&self, view: &mut View, [x, y, z]: [i32; 3], random: &mut Mt19937) {
        let (low, high) = self.offset;
        let top = self.slope(high);
        for dy in low..=high {
            let radius = top + self.min_width - self.slope(dy);
            if radius < 0 {
                continue;
            }
            for cx in x - radius..=x + radius {
                for cz in z - radius..=z + radius {
                    if (cx - x).abs() == radius && (cz - z).abs() == radius && self.cut_corner(dy - low, random) {
                        continue;
                    }
                    put(view, [cx, y + dy, cz], self.leaf, leaf_replaceable);
                }
            }
        }
        let Some(decoration) = &self.decoration else { return };
        for dy in low..=high {
            let radius = self.slope(high - dy) + self.min_width;
            for cx in x - radius..=x + radius {
                for cz in z - radius..=z + radius {
                    if view.block(cx, y + dy, cz) == self.leaf {
                        decoration.place(view, [cx, y + dy, cz], random);
                    }
                }
            }
        }
    }
}

fn acacia(view: &mut View, [x, y, z]: [i32; 3], leaf: Block, size: i32, simplify: bool, may_replace: &[Block]) {
    let gate = |block| allows(may_replace, block);
    let m = size.max(1);
    if simplify {
        for dx in 1 - m..=m - 1 {
            for dz in 1 - m..=m - 1 {
                put(view, [x + dx, y + 1, z + dz], leaf, gate);
            }
        }
    } else {
        let n = size.max(2);
        for dx in 2 - n..=n - 2 {
            for dz in 2 - n..=n - 2 {
                put(view, [x + dx, y + 1, z + dz], leaf, gate);
            }
        }
        for (dx, dz) in [(m - 1, 0), (1 - m, 0), (0, m - 1), (0, 1 - m)] {
            put(view, [x + dx, y + 1, z + dz], leaf, gate);
        }
    }
    ring(view, [x, y, z], size, true, leaf, &gate);
}

/// Rings from one above the anchor down `height` layers, growing to `top_cap`, one step back two
/// layers from the bottom.
fn pine(view: &mut View, [x, y, z]: [i32; 3], leaf: Block, height: i32, top_cap: i32, may_replace: &[Block]) {
    let gate = |block| allows(may_replace, block);
    let mut radius = 0;
    let mut dy = 1;
    loop {
        ring(view, [x, y + dy, z], radius, false, leaf, &gate);
        let grown = if radius < top_cap { radius + 1 } else { radius };
        let done = dy - 1 == -height;
        radius = if radius > 0 && dy == 2 - height { radius - 1 } else { grown };
        dy -= 1;
        if done {
            break;
        }
    }
}

/// Tiers growing from a drawn radius to a cap, resetting each time the cap is reached.
fn spruce(view: &mut View, [x, y, z]: [i32; 3], random: &mut Mt19937, leaf: Block, ranges: (super::spec::Range, super::spec::Range, super::spec::Range), may_grow_on: &[Block]) {
    let (lower_offset, upper_offset, max_radius) = ranges;
    let mut ground = y;
    while !allows(may_grow_on, view.block(x, ground - 1, z)) {
        if ground <= MIN_Y {
            return;
        }
        ground -= 1;
    }
    let lower = lower_offset.draw(random);
    let upper = upper_offset.draw(random);
    let cap = max_radius.draw(random);
    let mut radius = random.next_int(max_radius.1 - max_radius.0);
    let layers = upper + y - lower - ground;
    if layers <= -1 {
        return;
    }
    let mut width_cap = max_radius.1 - max_radius.0 - 1;
    let mut reset_to = 0;
    for dy in 0..=layers {
        ring(view, [x, y + upper - dy, z], radius, false, leaf, &leaf_replaceable);
        let new_cap = if width_cap < cap { width_cap + 1 } else { cap };
        let growing = radius < width_cap;
        radius = if growing { radius + 1 } else { reset_to };
        if !growing {
            width_cap = new_cap;
            reset_to = 1;
        }
    }
}

/// Discs of `radius` (one less on the first and last layer) where `(|dx|+.5)^2 + (|dz|+.5)^2 <= r^2`.
fn fancy(view: &mut View, [x, y, z]: [i32; 3], leaf: Block, height: i32, radius: i32, may_replace: &[Block]) {
    let gate = |block| allows(may_replace, block);
    for dy in 0..height {
        let r = if dy == 0 || dy == height - 1 { radius - 1 } else { radius };
        let rr = (r * r) as f64;
        for dx in -r..=r {
            let fx = dx.abs() as f32 as f64 + 0.5;
            for dz in -r..=r {
                let fz = dz.abs() as f64 + 0.5;
                if fx * fx + fz * fz <= rr {
                    put(view, [x + dx, y + dy, z + dz], leaf, gate);
                }
            }
        }
    }
}
