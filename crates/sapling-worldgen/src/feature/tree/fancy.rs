//! `fancy_trunk` (fancy oaks): foliage clusters scattered on a semicircular profile, a canopy at
//! each, the trunk, then a limb to each cluster high enough. featurelab `placeFancyTrunk`; the
//! limbs' pillar axis follows Java's rule (BDS palettes show x/z oak logs; featurelab has none).

use super::Anchor;
use super::common::{allows, log_along, valid_tree_position};
use super::spec::{FancyTrunk, Tree};
use crate::block::Block;
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::mth;

/// The largest magnitude of the three components, truncated: one cell per step on the main axis.
fn steps([dx, dy, dz]: [i32; 3]) -> i32 {
    (dx as f32).abs().max((dy as f32).abs()).max((dz as f32).abs()) as i32
}

/// The cells of the line from `from` towards `to`, both included, rounded as BDS does per step
/// (`from + trunc(i * delta / steps + 0.5)` in f32).
fn line(from: [i32; 3], to: [i32; 3]) -> impl Iterator<Item = [i32; 3]> {
    let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let n = steps(delta);
    let step = delta.map(|d| if n != 0 { d as f32 / n as f32 } else { 0.0 });
    (0..=n).map(move |i| std::array::from_fn(|a| from[a] + (i as f32 * step[a] + 0.5) as i32))
}

/// The index of the first cell on the line that is no valid tree position, None when all are.
fn blocked(view: &View, from: [i32; 3], to: [i32; 3], may_replace: &[Block]) -> Option<i32> {
    line(from, to).position(|p| !valid_tree_position(view, p, may_replace)).map(|i| i as i32)
}

fn limb(view: &mut View, from: [i32; 3], to: [i32; 3], log: Block) {
    let block = log_along(log, to[0] - from[0], to[2] - from[2]);
    for [x, y, z] in line(from, to) {
        view.set_block(x, y, z, block);
    }
}

impl FancyTrunk {
    pub fn place(&self, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3]) -> Option<[i32; 3]> {
        let mut height = self.height_base + ctx.random.next_int(self.height_variance);
        let origin = tree.roots_then(ctx, origin, height)?;
        if height < 1 {
            return None;
        }
        let [ox, oy, oz] = origin;
        // No build-height guard: BDS's fancy trunk has none (featurelab).
        if !allows(tree.may_grow_on, ctx.view.block(ox, oy - 1, oz)) {
            return None;
        }
        if let Some(at) = blocked(ctx.view, origin, [ox, oy + height - 1, oz], tree.may_replace) {
            if at <= self.height_base {
                return None;
            }
            height = at;
        }
        let stem_top = oy + ((height as f32 * self.height_scale) as i32).min(height - 1);
        let span = self.height_variance + 1;
        let ratio = self.branch_density * height as f32 / span as f32;
        let clusters = ((ratio as f64).powi(2) as f32 + 1.382) as i32;
        let clusters = clusters.max(1);
        let rise = span / 3;
        let top_level = height - rise;
        let mut coords = vec![([ox, oy + top_level, oz], stem_top)];
        for level in (0..=top_level).rev() {
            if (level as f32) < height as f32 * self.foliage_altitude_factor {
                continue;
            }
            let half = height as f32 / 2.0;
            let d = half - level as f32;
            let profile = if d == 0.0 {
                half
            } else if d.abs() < half {
                ((half * half - d * d) as f64).sqrt() as f32
            } else {
                0.0
            };
            let radius = profile * 0.5;
            for _ in 0..clusters {
                let distance = self.width_scale * radius * (ctx.random.next_float() + 0.328);
                let angle = ctx.random.next_float() * 2.0 * std::f32::consts::PI;
                let cx = ox + (distance * mth::sin(angle) + 0.5) as i32;
                let cz = oz + (distance * mth::cos(angle) + 0.5) as i32;
                let cluster = [cx, oy + level - 1, cz];
                if blocked(ctx.view, cluster, [cx, cluster[1] + rise, cz], tree.may_replace).is_some() {
                    continue;
                }
                let (hx, hz) = (ox - cx, oz - cz);
                let horizontal = ((hx * hx + hz * hz) as f64).sqrt() as i32;
                let dropped = cluster[1] as f32 - horizontal as f32 * self.branch_slope;
                let attach = if dropped <= stem_top as f32 { dropped as i32 } else { stem_top };
                if blocked(ctx.view, [ox, attach, oz], cluster, tree.may_replace).is_some() {
                    continue;
                }
                coords.push((cluster, attach));
            }
        }
        let anchors: Vec<Anchor> = coords.iter().map(|&(at, _)| Anchor::at(at)).collect();
        tree.place_canopies(ctx, &anchors);
        for i in 0..self.width {
            for j in 0..self.width {
                limb(ctx.view, [ox + i, oy, oz + j], [ox + i, stem_top, oz + j], self.block);
            }
        }
        for &(cluster, attach) in &coords {
            let from_x = if cluster[0] > ox { ox + self.width - 1 } else { ox };
            let from_z = if cluster[2] > oz { oz + self.width - 1 } else { oz };
            if (attach - oy) as f32 >= height as f32 * self.min_altitude_factor {
                limb(ctx.view, [from_x, attach, from_z], cluster, self.block);
            }
        }
        Some(origin)
    }
}
