//! `roofed_canopy` (dark and pale oaks): a `2r+1` floor under the anchor, walls of the inner
//! radius `height - 3` layers tall, a chamfered roof cap and, on a coin flip, a peak.
//! featurelab `roofedCanopy.place`; the leaf gate is air alone.

use super::Anchor;
use super::spec::{Canopy, Tree};
use crate::block::Block;
use crate::feature::view::View;
use crate::random::Mt19937;

/// The roof cap's two-clause corner exclusion, kept as BDS writes it.
fn upper_corner_allowed(j: i32, k: i32, outer: i32) -> bool {
    !(j == -outer && k <= 1 - outer || k == -outer && j == 1 - outer)
}

/// `Canopy::Roofed`.
pub fn canopy(canopy: &Canopy, _tree: &Tree, view: &mut View, random: &mut Mt19937, anchor: &Anchor) {
    let &Canopy::Roofed { leaf, height, core_width, outer_radius, inner_radius } = canopy else { return };
    let [x, y, z] = anchor.at;
    let core = core_width.max(1);
    let h = height - 3;
    let put =|view: &mut View, px: i32, py: i32, pz: i32| {
        if view.block(px, py, pz) == Block::Air {
            view.set_block(px, py, pz, leaf);
        }
    };
    if outer_radius != -1 {
        for j in -outer_radius..=0 {
            for k in -outer_radius..=0 {
                let (mj, mk) = (core - 1 - j, core - 1 - k);
                for (dx, dz) in [(j, k), (mj, k), (j, mk), (mj, mk)] {
                    put(view, x + dx, y - 1, z + dz);
                }
                if upper_corner_allowed(j, k, outer_radius) {
                    for (dx, dz) in [(j, k), (mj, k), (j, mk), (mj, mk)] {
                        put(view, x + dx, y + h, z + dz);
                    }
                }
            }
        }
    }
    if random.next_boolean() {
        for dx in 0..core {
            for dz in 0..core {
                put(view, x + dx, y + h + 1, z + dz);
            }
        }
    }
    let r = inner_radius;
    let far = core - 1 + r;
    for layer in 0..h.max(0) {
        for dx in -r..=far {
            for dz in -r..=far {
                let corner = (dx == -r || dx == far) && (dz == -r || dz == far);
                if corner || dx.abs() >= r && dz.abs() >= r {
                    continue;
                }
                put(view, x + dx, y + layer, z + dz);
            }
        }
    }
}
