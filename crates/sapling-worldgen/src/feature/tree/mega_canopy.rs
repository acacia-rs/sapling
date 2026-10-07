//! `mega_canopy` (a cone narrowing upwards) and `mega_pine_canopy` (a stepped taper), both laid
//! as layers of the radial block group.

use super::Anchor;
use super::common::allows;
use super::spec::{Canopy, Tree};
use crate::block::Block;
use crate::feature::view::View;
use crate::random::Mt19937;

/// `Canopy::Mega` and `Canopy::MegaPine`.
pub fn canopy(canopy: &Canopy, tree: &Tree, view: &mut View, random: &mut Mt19937, anchor: &Anchor) {
    let [x, y, z] = anchor.at;
    match *canopy {
        Canopy::Mega { leaf, height, base_radius, core_width, simplify } => {
            let value = height.draw(random);
            for i in 0..value.max(0) {
                let dy = 1 - value + i;
                let radius = value - 1 - i + base_radius + core_width / 2;
                radial(view, [x, y + dy, z], leaf, radius, core_width.max(1), simplify, tree.may_replace);
            }
        }
        Canopy::MegaPine { leaf, height, base_radius, radius_step_modifier, core_width } => {
            let value = height.draw(random);
            let mut previous = 0;
            for dy in -value..=0 {
                let ly = y + dy;
                let (radius, pre_bump) = if dy == 0 {
                    (base_radius, base_radius)
                } else {
                    let ratio = (-dy) as f32 / value as f32;
                    let pre_bump = base_radius + (radius_step_modifier * ratio).floor() as i32;
                    // A plateau in the taper juts out one more on even world y.
                    let bump = i32::from(pre_bump == previous && ly & 1 == 0);
                    (pre_bump + bump, pre_bump)
                };
                previous = pre_bump;
                radial(view, [x, ly, z], leaf, radius, core_width.max(1), false, tree.may_replace);
            }
        }
        _ => {}
    }
}

/// The radial block group: over the core square widened by `radius`, cells with a corner of their
/// unit square within `radius` of the near corner (only the cell itself when `strict`).
fn radial(view: &mut View, [x, y, z]: [i32; 3], leaf: Block, radius: i32, core: i32, strict: bool, may_replace: &[Block]) {
    if radius < 0 {
        return;
    }
    let within = |dx: i32, dz: i32| dx * dx + dz * dz <= radius * radius;
    for dx in -radius..=core - 1 + radius {
        for dz in -radius..=core - 1 + radius {
            let keep = within(dx, dz) || !strict && (within(dx - 1, dz - 1) || within(dx, dz - 1) || within(dx - 1, dz));
            if keep && allows(may_replace, view.block(x + dx, y, z + dz)) {
                view.set_block(x + dx, y, z + dz, leaf);
            }
        }
    }
}
