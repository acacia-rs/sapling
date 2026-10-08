//! `LakeFeature::place` (149e923b0) as a feature (the sulfur caves' `sulfur_pool`): the legacy
//! lake's blobs (decoration/lake.rs) checked against the view, filled with the liquid under air,
//! and its solid border turned into the border block, upper half by `nextInt(2)`.

use super::place::Ctx;
use crate::block::Block;
use crate::decoration::lake;

#[derive(Debug)]
pub struct Pool {
    pub liquid: Block,
    pub border: Block,
    /// Border blocks (by name) that cancel the lake.
    pub avoid: &'static [Block],
}

impl Pool {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        let view = &*ctx.view;
        let lake = lake::shape(ctx.random, at, &mut |x, y, z| view.block(x, y, z) == Block::Air)?;
        let [x0, y0, z0] = lake.corner();
        let cell = |x: i32, z: i32, y: i32| [x0 + x, y0 + y, z0 + z];
        for x in 0..16 {
            for z in 0..16 {
                for y in 0..8 {
                    if lake.border(x, z, y) && !self.holds(ctx, cell(x, z, y), y) {
                        return None;
                    }
                }
            }
        }
        for x in 0..16 {
            for z in 0..16 {
                for y in 0..8 {
                    if lake.filled(x, z, y) {
                        let [px, py, pz] = cell(x, z, y);
                        ctx.view.set_block(px, py, pz, if y < 4 { self.liquid } else { Block::Air });
                    }
                }
            }
        }
        for x in 0..16 {
            for z in 0..16 {
                for y in 0..8 {
                    if !lake.border(x, z, y) || (y >= 4 && ctx.random.next_int(2) == 0) {
                        continue;
                    }
                    let [px, py, pz] = cell(x, z, y);
                    if ctx.view.block(px, py, pz).is_solid() {
                        ctx.view.set_block(px, py, pz, self.border);
                    }
                }
            }
        }
        Some(at)
    }

    /// A border cell takes no liquid in the upper half, and in the lower is solid or the liquid.
    fn holds(&self, ctx: &Ctx, [x, y, z]: [i32; 3], local_y: i32) -> bool {
        let block = ctx.view.block(x, y, z);
        let liquid = block.is_fluid();
        let same = liquid && block.is_lava() == self.liquid.is_lava();
        if local_y >= 4 && liquid || local_y < 4 && !block.is_solid() && !same {
            return false;
        }
        !self.avoid.iter().any(|a| a.name() == block.name())
    }
}
