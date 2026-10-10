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
    block == Block::Air || (block.is_fluid() && !block.is_lava())
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
                    bubble_column(ctx.view, p);
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

/// One bubble column cell in the water source above; the column's ticks wait for the game
/// (docs/research/worldgen-bds-liquids.md, "Game time"), so it grows only under a game-tick replay.
fn bubble_column(view: &mut View, [x, mut y, z]: [i32; 3]) {
    let cells = if super::liquid::game_ticks() > 0 { usize::MAX } else { 1 };
    for _ in 0..cells {
        y += 1;
        if view.block(x, y, z) != Block::Water {
            break;
        }
        view.set_block(x, y, z, Block::BubbleColumn);
    }
}
