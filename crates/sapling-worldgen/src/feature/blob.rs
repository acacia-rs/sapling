//! `minecraft:partially_exposed_blob_feature` after stirante/featurelab
//! `features/partially_exposed_blob.go` (MIT, (c) 2026 stirante): around the block under the
//! position, ring by ring, one `nextFloat` per cell, the block where it is enclosed but for one
//! face. Magma under water starts a bubble column.

use super::cave_spec::PartiallyExposedBlob;
use super::place::Ctx;
use super::view::View;
use crate::block::Block;

const FACING: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

fn open(block: Block) -> bool {
    // The bubble column an earlier cell of the blob started is still water to the test (temple replay).
    // So is the water a lichen or a plant stands in (frozen ocean one-worker run: no magma beside a wet lichen).
    block == Block::Air || (block.is_fluid() && !block.is_lava()) || block == Block::BubbleColumn || block.is_waterlogged()
}

/// Offsets by ring (|x| + |y| + |z|), x then y ascending, +z before -z.
fn offsets(radius: i32) -> Vec<[i32; 3]> {
    let mut out = Vec::new();
    for ring in 0..=3 * radius {
        let bx = ring.min(radius);
        for dx in -bx..=bx {
            let by = (ring - dx.abs()).min(radius);
            for dy in -by..=by {
                let dz = ring - dx.abs() - dy.abs();
                if dz > radius {
                    continue;
                }
                out.push([dx, dy, dz]);
                if dz != 0 {
                    out.push([dx, dy, -dz]);
                }
            }
        }
    }
    out
}

impl PartiallyExposedBlob {
    pub fn place(&self, ctx: &mut Ctx, [x, y, z]: [i32; 3]) -> Option<[i32; 3]> {
        let mut placed = false;
        for [dx, dy, dz] in offsets(self.radius) {
            let p = [x + dx, y - 1 + dy, z + dz];
            if ctx.random.next_float() <= self.probability && self.enclosed(ctx.view, p) {
                ctx.view.set_block(p[0], p[1], p[2], self.block);
                if self.block == Block::Magma {
                    // The magma block's own tick, in 1, starts the bubble column (liquid/flow.rs, `magma_tick`). It
                    // waits on its chunk's queue: for that chunk's decoration, or for the game if that is over.
                    ctx.view.ticks.in_game.push((p, false, 1));
                }
                placed = true;
            }
        }
        placed.then_some([x, y, z])
    }

    fn enclosed(&self, view: &View, [x, y, z]: [i32; 3]) -> bool {
        !open(view.block(x, y, z))
            && FACING.iter().enumerate().all(|(face, [dx, dy, dz])| face == self.exposed || !open(view.block(x + dx, y + dy, z + dz)))
    }
}
