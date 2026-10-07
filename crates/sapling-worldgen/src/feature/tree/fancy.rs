//! `fancy_trunk` (fancy oaks), `FancyTreeTrunk::placeTrunk` (1438ec690): the trunk line shortened
//! at the first cell it cannot grow through, foliage clusters on a semicircular profile each with
//! a clear line to the trunk, a canopy at each, the trunk, then a limb to each high enough. Limb
//! logs lie along the axis of each cell's offset from the limb's start (upright where it is 0).

use super::Anchor;
use super::common::log_along;
use super::spec::{FancyTrunk, Tree};
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::mth;

/// The cells of the line from `from` by `delta`, `max(|delta|) + 1` of them, each `from +
/// trunc(i * d / n + 0.5)` in f32, with the horizontal part of that offset.
fn line(from: [i32; 3], delta: [i32; 3]) -> impl Iterator<Item = ([i32; 3], [i32; 2])> {
    let n = delta.iter().map(|d| d.abs()).max().unwrap_or(0);
    let step = delta.map(|d| d as f32 / n as f32);
    // A zero-length line divides by zero in BDS and lands at y = i32::MIN: no cell.
    let last = if n == 0 { -1 } else { n };
    (0..=last).map(move |i| {
        let offset: [i32; 3] = std::array::from_fn(|a| (i as f32 * step[a] + 0.5) as i32);
        (std::array::from_fn(|a| from[a] + offset[a]), [offset[0], offset[2]])
    })
}

/// Whether every cell of the line can be grown through; an empty `may_grow_through` allows all.
fn clear(tree: &Tree, view: &View, from: [i32; 3], delta: [i32; 3]) -> bool {
    tree.may_grow_through.is_empty() || line(from, delta).all(|([x, y, z], _)| tree.grows_through(view.block(x, y, z)))
}

impl FancyTrunk {
    pub fn place(&self, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3]) -> Option<[i32; 3]> {
        let mut height = self.height_base + ctx.random.next_int(self.height_variance);
        let origin = tree.roots_then(ctx, origin, height)?;
        let [ox, oy, oz] = origin;
        if !tree.may_grow_on.is_empty() && !tree.may_grow_on.contains(&ctx.view.block(ox, oy - 1, oz)) {
            return None;
        }
        if !tree.may_grow_through.is_empty()
            && let Some(at) = line(origin, [0, height - 1, 0]).position(|([x, y, z], _)| !tree.grows_through(ctx.view.block(x, y, z)))
        {
            if at as i32 <= self.height_base {
                return None;
            }
            height = at as i32;
        }
        let h = height as f32;
        let stem = (self.height_scale * h) as i32;
        let stem_top = oy + if stem < height { stem } else { height - 1 };
        let span = self.height_variance + 1;
        let ratio = self.branch_density * h / span as f32;
        let clusters = ((ratio * ratio + 1.382) as i32).max(1);
        let rise = span / 3;
        let mut coords = vec![([ox, oy + height - rise, oz], stem_top)];
        let half = h * 0.5;
        for level in (0..=height - rise).rev() {
            if self.foliage_altitude_factor * h > level as f32 {
                continue;
            }
            let d = half - level as f32;
            let profile = if d == 0.0 {
                half
            } else if d.abs() < half {
                (half * half - d * d).sqrt()
            } else {
                0.0
            };
            let reach = self.width_scale * (profile * 0.5);
            for _ in 0..clusters {
                let near = ctx.random.next_float();
                let turn = ctx.random.next_float();
                let distance = (near + 0.328) * reach;
                let angle = (turn + turn) * std::f32::consts::PI;
                let cx = ox + (distance * mth::sin(angle) + 0.5) as i32;
                let cz = oz + (distance * mth::cos(angle) + 0.5) as i32;
                let cluster = [cx, oy + level - 1, cz];
                if !clear(tree, ctx.view, cluster, [0, rise, 0]) {
                    continue;
                }
                let (hx, hz) = (ox - cx, oz - cz);
                let horizontal = ((hz * hz + hx * hx) as f64).sqrt() as i32 as f32;
                let dropped = cluster[1] as f32 - horizontal * self.branch_slope;
                let attach = if dropped > stem_top as f32 { stem_top } else { dropped as i32 };
                if !clear(tree, ctx.view, [ox, attach, oz], [cx - ox, cluster[1] - attach, cz - oz]) {
                    continue;
                }
                coords.push((cluster, attach));
            }
        }
        let anchors: Vec<Anchor> = coords.iter().map(|&(at, _)| Anchor::at(at)).collect();
        tree.place_canopies(ctx, &anchors);
        for i in 0..self.width {
            for j in 0..self.width {
                self.logs(ctx.view, [ox + i, oy, oz + j], [0, stem_top - oy, 0]);
            }
        }
        for &(cluster, attach) in &coords {
            let from_x = if cluster[0] > ox { ox + self.width - 1 } else { ox };
            let from_z = if cluster[2] > oz { oz + self.width - 1 } else { oz };
            if self.min_altitude_factor * h <= (attach - oy) as f32 {
                self.logs(ctx.view, [from_x, attach, from_z], [cluster[0] - from_x, cluster[1] - attach, cluster[2] - from_z]);
            }
        }
        Some(origin)
    }

    fn logs(&self, view: &mut View, from: [i32; 3], delta: [i32; 3]) {
        for ([x, y, z], [dx, dz]) in line(from, delta) {
            view.set_block(x, y, z, log_along(self.block, dx, dz));
        }
    }
}
