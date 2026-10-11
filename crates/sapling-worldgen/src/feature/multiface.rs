//! `MultifaceFeature::place` (143b83e50, attach 143b84880) and `SurfaceRelativeThresholdFeature`
//! (143bc69b0). BDS shuffles the faces and a spread's six directions (1465e9360) with its worker's
//! thread-local generator (thread.rs), which a decoration always has here: seeded per chunk, or a
//! logged run's (docs/research/worldgen-bds-order.md, "One engine"). A structure piece's feature has
//! none and keeps the pushed order: lichen then spreads the first way that can hold it, a vein only
//! when one way can.

use super::cave_spec::{Multiface, SurfaceThreshold};
use super::data::FEATURES;
use super::place::Ctx;
use super::spreader::{self, Config, placement, step};
use crate::block::Block;
use crate::density::NO_SURFACE;

fn open(block: Block) -> bool {
    block == Block::Air || (block.is_fluid() && !block.is_lava())
}

/// What any face order could make of a call, for reach.rs: the (cell, face) placements, whether
/// some order places nothing, and the cells a spread from a placement could take.
struct Orders {
    placements: Vec<([i32; 3], usize)>,
    may_fail: bool,
    spreads: Vec<[i32; 3]>,
}

impl Multiface {
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        if !open(ctx.view.block(at[0], at[1], at[2])) {
            return None;
        }
        let orders = ctx.view.reach_on().then(|| self.orders(ctx, at));
        let placed = if ctx.view.thread_random.is_some() { self.place_shuffled(ctx, at) } else { self.place_in_order(ctx, at) };
        if let Some(orders) = orders {
            orders.note(ctx, placed.is_some_and(|(_, spreads)| spreads));
        }
        placed.map(|(cell, _)| cell)
    }

    /// The cell placed in and whether the spread gate passed.
    fn place_in_order(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<([i32; 3], bool)> {
        if let Some(spreads) = self.attach(ctx, at, self.faces) {
            return Some((at, spreads));
        }
        for &facing in self.faces {
            let next = step(at, facing);
            if !self.searches(ctx, next) {
                continue;
            }
            // BDS retries `search_range` times at the same cell; the outcome cannot change.
            if let Some(spreads) = self.attach(ctx, next, &self.faces_from(facing)) {
                return Some((next, spreads));
            }
        }
        None
    }

    /// Whether the search tries the neighbour cell: open or holding the feature's block.
    pub(super) fn searches(&self, ctx: &Ctx, next: [i32; 3]) -> bool {
        let block = ctx.view.block(next[0], next[1], next[2]);
        self.search_range > 0 && (open(block) || self.is_own(block))
    }

    /// The faces tried in the neighbour toward `facing`: all but the one back to the origin.
    pub(super) fn faces_from(&self, facing: usize) -> Vec<usize> {
        self.faces.iter().copied().filter(|&f| f != facing ^ 1).collect()
    }

    fn family(&self) -> Block {
        self.block.multiface().map_or(self.block, |(family, _)| family)
    }

    fn is_own(&self, block: Block) -> bool {
        block.multiface().is_some_and(|(family, _)| family == self.family())
    }

    /// Whether the block behind `face` of the cell `at` is one the feature places on.
    fn takes(&self, ctx: &Ctx, at: [i32; 3], face: usize) -> bool {
        let [x, y, z] = step(at, face);
        let on = ctx.view.block(x, y, z);
        if !(self.can_place_on.is_empty() || self.can_place_on.contains(&on) || matches!(on, Block::Template(_)) && self.on_names.contains(&on.name())) {
            return false;
        }
        // 143b84a00: no face on a tunnel structure's block in another chunk not decorated yet.
        let (source, other) = (ctx.view.chunk(), (x >> 4, z >> 4));
        other == (at[0] >> 4, at[2] >> 4) || super::phase(other) < super::phase(source) && other != source || !(ctx.in_tunnels)([x, y, z])
    }

    /// The first face whose neighbour `can_place_on` takes decides: the block gains it when that
    /// face is sturdy and the cell changes (then the spread gate is drawn and returned), else a failure.
    pub(super) fn attach(&self, ctx: &mut Ctx, at: [i32; 3], faces: &[usize]) -> Option<bool> {
        let existing = ctx.view.block(at[0], at[1], at[2]);
        let &face = faces.iter().find(|&&face| self.takes(ctx, at, face))?;
        let placed = placement(ctx.view, self.family(), at, existing, face);
        if placed == existing {
            return None;
        }
        ctx.view.set_block(at[0], at[1], at[2], placed);
        let spreads = ctx.random.next_float() < self.spread_chance;
        if spreads {
            self.spread_one_way(ctx, at, placed, face);
        }
        Some(spreads)
    }

    fn config(&self) -> Config {
        if self.family() == Block::GlowLichen1 { Config::Lichen } else { Config::Vein }
    }

    fn spread_one_way(&self, ctx: &mut Ctx, at: [i32; 3], state: Block, from: usize) {
        let config = self.config();
        if ctx.view.thread_random.is_some() {
            return super::thread::spread(config, ctx, at, state, from);
        }
        super::trace::line(|| {
            let found = (0..6).filter_map(|to| spreader::spread_target(config, ctx.view, state, at, from, to).map(|(c, face, _)| format!("{to}:{},{},{},{face}", c[0], c[1], c[2])));
            format!("    spread {} {} {} from {from} to {}", at[0], at[1], at[2], found.collect::<Vec<_>>().join(" "))
        });
        let mut targets = (0..6).filter_map(|to| spreader::spread_target(config, ctx.view, state, at, from, to));
        let (first, second) = (targets.next(), targets.next());
        // BDS always spreads; a guessed vein scored worse on the oracle than none, a guessed lichen better.
        if let (Some((cell, _, block)), true) = (first, second.is_none() || config == Config::Lichen) {
            ctx.view.set_block(cell[0], cell[1], cell[2], block);
        }
    }

    /// Each face a cell's `attach` could take first, and whether it would place.
    fn firsts(&self, ctx: &Ctx, at: [i32; 3], faces: &[usize]) -> Vec<(usize, bool)> {
        let existing = ctx.view.block(at[0], at[1], at[2]);
        let places = |face| placement(ctx.view, self.family(), at, existing, face) != existing;
        faces.iter().filter(|&&face| self.takes(ctx, at, face)).map(|&face| (face, places(face))).collect()
    }

    fn orders(&self, ctx: &Ctx, at: [i32; 3]) -> Orders {
        let origin = self.firsts(ctx, at, self.faces);
        let mut placements: Vec<_> = origin.iter().filter(|&&(_, places)| places).map(|&(face, _)| (at, face)).collect();
        let mut may_fail = false;
        if origin.iter().any(|&(_, places)| !places) || origin.is_empty() {
            // A facing surely places when every face its cell could take first does.
            let mut sure = false;
            for &facing in self.faces {
                let next = step(at, facing);
                if !self.searches(ctx, next) {
                    continue;
                }
                let firsts = self.firsts(ctx, next, &self.faces_from(facing));
                sure |= !firsts.is_empty() && firsts.iter().all(|&(_, places)| places);
                placements.extend(firsts.iter().filter(|&&(_, places)| places).map(|&(face, _)| (next, face)));
            }
            may_fail = !sure;
        }
        let mut spreads = Vec::new();
        for &(cell, face) in &placements {
            let state = placement(ctx.view, self.family(), cell, ctx.view.block(cell[0], cell[1], cell[2]), face);
            spreads.extend((0..6).filter_map(|to| spreader::spread_target(self.config(), ctx.view, state, cell, face, to)).map(|(target, ..)| target));
        }
        Orders { placements, may_fail, spreads }
    }
}

impl Orders {
    /// Leaves in doubt what the face order decides; `spread`: our call placed and passed the gate.
    fn note(self, ctx: &mut Ctx, spread: bool) {
        if self.may_fail && !self.placements.is_empty() {
            ctx.view.reach_taint();
            return;
        }
        let several = self.placements.len() > 1;
        if several {
            self.placements.iter().for_each(|&(cell, _)| ctx.view.reach_doubt(cell));
        }
        if spread && (several || self.spreads.len() > 1) {
            self.spreads.into_iter().for_each(|cell| ctx.view.reach_doubt(cell));
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
