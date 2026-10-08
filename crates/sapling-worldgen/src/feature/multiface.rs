//! `MultifaceFeature::place` (143b83e50, attach 143b84880) and `SurfaceRelativeThresholdFeature`
//! (143bc69b0). BDS shuffles the faces with a thread-local generator (nondeterministic: the
//! oracle's two runs disagree on them); this keeps their pushed order, so which face a block takes
//! can differ but whether a cell gets one does not. Spreads use that generator too and are left
//! out; their `nextFloat` gate on the placing generator is drawn.

use super::cave_spec::{Multiface, SurfaceThreshold};
use super::data::FEATURES;
use super::place::Ctx;
use crate::block::Block;
use crate::density::NO_SURFACE;

/// FacingID offsets: down, up, north, south, west, east.
const FACING: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

fn step([x, y, z]: [i32; 3], facing: usize) -> [i32; 3] {
    let [dx, dy, dz] = FACING[facing];
    [x + dx, y + dy, z + dz]
}

fn open(block: Block) -> bool {
    block == Block::Air || (block.is_fluid() && !block.is_lava())
}

impl Multiface {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        if !open(ctx.view.block(at[0], at[1], at[2])) {
            return None;
        }
        if self.attach(ctx, at, self.faces) {
            return Some(at);
        }
        for &facing in self.faces {
            let next = step(at, facing);
            let block = ctx.view.block(next[0], next[1], next[2]);
            if !open(block) && !self.is_own(block) {
                continue;
            }
            let faces: Vec<usize> = self.faces.iter().copied().filter(|&f| f != facing ^ 1).collect();
            // BDS retries `search_range` times at the same cell; the outcome cannot change.
            if self.search_range > 0 && self.attach(ctx, next, &faces) {
                return Some(next);
            }
        }
        None
    }

    fn is_own(&self, block: Block) -> bool {
        (self.block as usize..self.block as usize + 6).contains(&(block as usize))
    }

    /// The first face whose neighbour `can_place_on` takes decides: the block on it when that face
    /// is sturdy and the cell changes (then the spread gate is drawn), else a failure.
    fn attach(&self, ctx: &mut Ctx, at: [i32; 3], faces: &[usize]) -> bool {
        let existing = ctx.view.block(at[0], at[1], at[2]);
        let Some(&face) = faces.iter().find(|&&face| {
            let [x, y, z] = step(at, face);
            self.can_place_on.is_empty() || self.can_place_on.contains(&ctx.view.block(x, y, z))
        }) else {
            return false;
        };
        let [sx, sy, sz] = step(at, face);
        let placed = if super::lush::sturdy(ctx.view.block(sx, sy, sz)) && !self.is_own(existing) { self.block.faced(face) } else { existing };
        if placed == existing {
            return false;
        }
        ctx.view.set_block(at[0], at[1], at[2], placed);
        ctx.random.next_float();
        true
    }
}

impl SurfaceThreshold {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        let surface = (ctx.surface)(at[0] >> 2, at[2] >> 2);
        if surface == NO_SURFACE || at[1] >= surface - self.min_depth {
            return None;
        }
        FEATURES[self.feature].place(ctx, at)
    }
}
