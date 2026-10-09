//! `minecraft:fossil_feature` (`FossilFeature::place` 149e82e00): one of eight bone templates
//! under the rule's chunk, nine tenths of its blocks placed and a tenth of them as the ore block
//! (docs/research/worldgen-bds-features.md, "Fossils").

use super::place::Ctx;
use super::trace;
use crate::block::Block;
use crate::structure::BoundingBox;
use crate::structure::template::{Placement, Rotation, Template};
use crate::terrain::MIN_Y;

/// The name table 149ea0080 fills, in its order.
const TEMPLATES: [&str; 8] = [
    "fossils/fossil_spine_01",
    "fossils/fossil_spine_02",
    "fossils/fossil_spine_03",
    "fossils/fossil_spine_04",
    "fossils/fossil_skull_01",
    "fossils/fossil_skull_02",
    "fossils/fossil_skull_03",
    "fossils/fossil_skull_04",
];
const UNBOUNDED: BoundingBox = BoundingBox { x0: i32::MIN, y0: i32::MIN, z0: i32::MIN, x1: i32::MAX, y1: i32::MAX, z1: i32::MAX };

#[derive(Debug)]
pub struct Fossil {
    pub ore: Block,
    pub max_empty_corners: i32,
}

impl Fossil {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> bool {
        if (ctx.in_structure)(at) {
            return false;
        }
        let rotation = Rotation::from_index(ctx.random.next_int(4));
        let template = Template::get(TEMPLATES[ctx.random.next_int(8) as usize]);
        let (sx, sy, sz) = template.size;
        let (wide, deep) = if matches!(rotation, Rotation::Clockwise90 | Rotation::CounterClockwise90) { (sz, sx) } else { (sx, sz) };
        let x = at[0] + ctx.random.next_int(16 - wide);
        let z = at[2] + ctx.random.next_int(16 - deep);
        let columns = (0..wide).flat_map(|dx| (0..deep).map(move |dz| (x + dx, z + dz)));
        let top = columns.map(|(cx, cz)| ctx.view.above_top_solid(cx, cz)).fold(at[1], i32::min);
        let y = (top - ctx.random.next_int(10) - 15).max(MIN_Y + 10);
        // The offsets fit the turned footprint into the chunk, but the template then turns about its corner: no pivot.
        let (fx, _, fz) = rotation.apply((sx - 1, 0, sz - 1));
        let corners = [x, x + fx].into_iter().flat_map(|cx| [y, y + sy - 1].into_iter().flat_map(move |cy| [z, z + fz].into_iter().map(move |cz| (cx, cy, cz))));
        let empty = corners.filter(|&(cx, cy, cz)| matches!(ctx.view.block(cx, cy, cz), block if block == Block::Air || block.is_fluid())).count();
        trace::line(|| format!("  fossil {} at {x} {y} {z} {rotation:?} empty corners {empty}", template.name));
        if empty as i32 > self.max_empty_corners {
            return false;
        }
        // Both passes draw integrity from the generator's seed afresh: the ore cells are the bone pass's lowest draws.
        for (integrity, ore) in [(0.9, None), (0.1, Some(self.ore))] {
            let mut placement = Placement::new(rotation, UNBOUNDED);
            placement.integrity = integrity;
            placement.seed = ctx.random.seed();
            for ((bx, by, bz), bone, _) in placement.blocks(template, (x, y, z)) {
                ctx.view.set_block(bx, by, bz, ore.unwrap_or(bone));
            }
        }
        true
    }
}
