//! `fallen_trunk`: a stump at the origin and, two or three blocks away, a log lying on the ground
//! (all or nothing), each lying log decorated by `log_decoration`.

use super::common::allows;
use super::spec::{Decoration, FallenTrunk, Tree};
use crate::block::Block;
use crate::feature::data::FEATURES;
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::terrain::MIN_Y;

/// North, south, west, east (`nextInt(4) + 2` as a facing).
const FACINGS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

/// Empty, or a block that can be built over (world-gen API slot 0x90): liquids too, so the log sinks to a pond's bed.
fn passable(view: &View, [x, y, z]: [i32; 3]) -> bool {
    let block = view.block(x, y, z);
    block == Block::Air
        || block.is_fluid()
        || matches!(
            block,
            Block::ShortGrass | Block::TallGrass | Block::TallGrassTop | Block::Fern | Block::LargeFern | Block::LargeFernTop | Block::Deadbush | Block::SnowLayer
        )
        || (Block::LeafLitter0South..=Block::LeafLitter3East).contains(&block)
}

impl FallenTrunk {
    pub fn place(&self, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3]) -> Option<[i32; 3]> {
        let length = self.log_length.draw(ctx.random);
        let origin = tree.roots_then(ctx, origin, length)?;
        if !tree.prepare(ctx.view, origin, length, 1) {
            return None;
        }
        let modifier = self.height_modifier.draw(ctx.random);
        let (dx, dz) = FACINGS[ctx.random.next_int(4) as usize];
        let offset = ctx.random.next_int(2) + 2;
        let logs = length + modifier - 2;
        let stump = self.stump_height.draw(ctx.random);
        let [x, y, z] = origin;
        let (sx, sz) = (x + dx * offset, z + dz * offset);
        let mut ly = y + stump;
        while ly > MIN_Y && passable(ctx.view, [sx, ly - 1, sz]) {
            ly -= 1;
        }
        if logs >= 1 && self.fits(ctx.view, [sx, ly, sz], (dx, dz), logs) {
            let block = if dx != 0 { self.block_x } else { self.block_z };
            for i in 0..logs {
                let p = [sx + dx * i, ly, sz + dz * i];
                ctx.view.set_block(p[0], p[1], p[2], block);
                if let Some(feature) = self.log_decoration {
                    FEATURES[feature].place(ctx, p);
                }
            }
        }
        let mut last = None;
        for i in 0..stump {
            let p = [x, y + i, z];
            if allows(tree.may_replace, ctx.view.block(p[0], p[1], p[2])) {
                let placed = Decoration::place_decorated(self.decoration.as_ref(), ctx.view, p, self.block, ctx.random);
                last = placed.then_some(p);
            }
        }
        tree.place_base_block(ctx.view, [x, y - 1, z]);
        last.map(|[px, py, pz]| [px, py + 1, pz])
    }

    /// Every cell passable, and never three cells in a row without solid ground under them.
    fn fits(&self, view: &View, [x, y, z]: [i32; 3], (dx, dz): (i32, i32), logs: i32) -> bool {
        let mut ok = true;
        let mut unsupported = 0;
        for i in 0..logs {
            let (cx, cz) = (x + dx * i, z + dz * i);
            if !passable(view, [cx, y, cz]) {
                ok = false;
            }
            if view.block(cx, y - 1, cz).blocks_motion() {
                unsupported = 0;
            } else {
                if unsupported >= 2 {
                    ok = false;
                }
                unsupported += 1;
            }
        }
        ok
    }
}
