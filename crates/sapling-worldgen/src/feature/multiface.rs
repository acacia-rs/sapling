//! `MultifaceFeature::place` (143b83e50, attach 143b84880) and `SurfaceRelativeThresholdFeature`
//! (143bc69b0). BDS shuffles the faces with a thread-local generator (nondeterministic: the
//! oracle's two runs disagree on them); this keeps their pushed order, so which face a block takes
//! can differ but whether a cell gets one does not. A spread (1465e9360) tries the directions in a
//! thread-local order too: lichen takes the first that can, a vein only a lone one
//! (docs/research/worldgen-bds-residue-lush.md).

use super::cave_spec::{Multiface, SurfaceThreshold};
use super::data::FEATURES;
use super::place::Ctx;
use super::spreader::{self, Config, placement, step};
use crate::block::Block;
use crate::density::NO_SURFACE;

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

    fn family(&self) -> Block {
        self.block.multiface().map_or(self.block, |(family, _)| family)
    }

    fn is_own(&self, block: Block) -> bool {
        block.multiface().is_some_and(|(family, _)| family == self.family())
    }

    /// The first face whose neighbour `can_place_on` takes decides: the block gains it when that
    /// face is sturdy and the cell changes (then the spread gate is drawn), else a failure.
    fn attach(&self, ctx: &mut Ctx, at: [i32; 3], faces: &[usize]) -> bool {
        let existing = ctx.view.block(at[0], at[1], at[2]);
        let source = super::phase(ctx.view.chunk());
        let Some(&face) = faces.iter().find(|&&face| {
            let [x, y, z] = step(at, face);
            let on = ctx.view.block(x, y, z);
            if !(self.can_place_on.is_empty() || self.can_place_on.contains(&on) || matches!(on, Block::Template(_)) && self.on_names.contains(&on.name())) {
                return false;
            }
            // 143b84a00: no face on a tunnel structure's block in another chunk not decorated yet.
            let other = (x >> 4, z >> 4);
            other == (at[0] >> 4, at[2] >> 4) || super::phase(other) < source && other != ctx.view.chunk() || !(ctx.in_tunnels)([x, y, z])
        }) else {
            return false;
        };
        let placed = placement(ctx.view, self.family(), at, existing, face);
        if placed == existing {
            return false;
        }
        ctx.view.set_block(at[0], at[1], at[2], placed);
        if ctx.random.next_float() < self.spread_chance {
            self.spread_one_way(ctx, at, placed, face);
        }
        true
    }

    fn spread_one_way(&self, ctx: &mut Ctx, at: [i32; 3], state: Block, from: usize) {
        let config = if self.family() == Block::GlowLichen1 { Config::Lichen } else { Config::Vein };
        super::trace::line(|| {
            let found = (0..6).filter_map(|to| spreader::spread_target(config, ctx.view, state, at, from, to).map(|(c, face, _)| format!("{to}:{},{},{},{face}", c[0], c[1], c[2])));
            format!("    spread {} {} {} from {from} to {}", at[0], at[1], at[2], found.collect::<Vec<_>>().join(" "))
        });
        let mut targets = (0..6).filter_map(|to| spreader::spread_target(config, ctx.view, state, at, from, to));
        let (first, second) = (targets.next(), targets.next());
        if let (Some((cell, _, block)), true) = (first, second.is_none() || config == Config::Lichen) {
            ctx.view.set_block(cell[0], cell[1], cell[2], block);
        }
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
