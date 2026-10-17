//! `NetherFossilFeature::place` (149d5bcb0): a bone template sunk down to soul sand, then perhaps a dried ghast
//! on soul soil inside it. Valid ground is `[soul_sand]`; it may not stand in lava.

use super::basalt::{BOTTOM, TOP};
use super::place::Ctx;
use super::trace;
use crate::block::Block;
use crate::structure::BoundingBox;
use crate::structure::template::{Placement, Rotation, Template};

/// The name table 149e815b0 fills, in its order.
const TEMPLATES: [&str; 14] = [
    "nether_fossils/fossil_1",
    "nether_fossils/fossil_2",
    "nether_fossils/fossil_3",
    "nether_fossils/fossil_4",
    "nether_fossils/fossil_5",
    "nether_fossils/fossil_6",
    "nether_fossils/fossil_7",
    "nether_fossils/fossil_8",
    "nether_fossils/fossil_9",
    "nether_fossils/fossil_10",
    "nether_fossils/fossil_11",
    "nether_fossils/fossil_12",
    "nether_fossils/fossil_13",
    "nether_fossils/fossil_14",
];
const UNBOUNDED: BoundingBox = BoundingBox { x0: i32::MIN, y0: i32::MIN, z0: i32::MIN, x1: i32::MAX, y1: i32::MAX, z1: i32::MAX };

pub fn place(ctx: &mut Ctx, [x, y0, z]: [i32; 3]) -> bool {
    let rotation = Rotation::from_index(ctx.random.next_int(4));
    let template = Template::get(TEMPLATES[ctx.random.next_int(14) as usize]);
    let (sx, sy, sz) = template.size;
    let (fx, _, fz) = rotation.apply((sx - 1, 0, sz - 1));
    let corners = [(x, z), (x + fx, z), (x, z + fz), (x + fx, z + fz)];
    let view = &mut *ctx.view;
    let mut y = y0;
    let y = loop {
        if y - 1 <= BOTTOM {
            return false;
        }
        y -= 1;
        if y + sy + 1 >= TOP {
            return false;
        }
        if corners.iter().any(|&(cx, cz)| view.block(cx, y, cz).is_lava()) {
            return false;
        }
        let fits = corners.iter().all(|&(cx, cz)| {
            let ground = view.block(cx, y, cz);
            view.block(cx, y + sy - 1, cz) == Block::Air && (ground.supports_multiface_plain(1) || ground == Block::SoulSand)
        });
        if fits {
            break y;
        }
    };
    trace::line(|| format!("  nether fossil {} at {x} {} {z} {rotation:?}", template.name, y + 1));
    let mut placement = Placement::new(rotation, UNBOUNDED);
    placement.seed = ctx.random.seed();
    for ((bx, by, bz), block, _) in placement.blocks(template, (x, y + 1, z)) {
        view.set_block(bx, by, bz, block);
    }
    if ctx.random.next_float() > 0.5 {
        return true;
    }
    let ghast = Block::dried_ghast(ctx.random.next_int(4));
    let (far_x, far_z) = (x + fx, z + fz);
    let gx = if x < far_x { x + ctx.random.next_int(far_x - x) } else { x };
    let gz = if z < far_z { z + ctx.random.next_int(far_z - z) } else { z };
    for gy in y + 1..y + 1 + sy {
        let there = view.block(gx, gy, gz);
        if there == Block::Air {
            if view.block(gx, gy - 1, gz) == Block::SoulSoil {
                view.set_block(gx, gy, gz, ghast);
            }
            break;
        }
        if there.name() == "bone_block" {
            break;
        }
    }
    true
}
