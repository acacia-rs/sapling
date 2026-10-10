//! `MultifaceFeature` with BDS's per-thread generator modelled: the face order (143b83e50) and the
//! spread's six directions (1403747f0) are forward shuffles drawn from it. Only meaningful against
//! a BDS run with one worker and a known seed (docs/research/worldgen-bds-order.md, "A repeatable BDS").

use super::cave_spec::Multiface;
use super::place::Ctx;
use super::spreader::{self, Config, step};
use crate::block::Block;
use crate::random::Mt19937;

/// For i in 1..n: swap with `raw % (i + 1)`.
pub fn shuffle<T>(list: &mut [T], random: &mut Mt19937) {
    for i in 1..list.len() {
        list.swap(i, random.next_u32() as usize % (i + 1));
    }
}

fn thread<'c>(ctx: &'c mut Ctx) -> &'c mut Mt19937 {
    ctx.view.thread_random.as_mut().expect("the thread generator is modelled")
}

impl Multiface {
    pub(super) fn place_shuffled(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        let before = thread(ctx).draws();
        let placed = self.place_drawn(ctx, at);
        let after = thread(ctx).draws();
        super::trace::line(|| format!("    thread {} {} {} draws {before}..{after} placed {placed:?}", at[0], at[1], at[2]));
        placed
    }

    fn place_drawn(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        let mut faces = self.faces.to_vec();
        shuffle(&mut faces, thread(ctx));
        if self.attach(ctx, at, &faces).is_some() {
            return Some(at);
        }
        for &facing in &faces {
            // BDS shuffles the neighbour's own list and then passes the first one.
            shuffle(&mut self.faces_from(facing), thread(ctx));
            let next = step(at, facing);
            if self.searches(ctx, next) && self.attach(ctx, next, &faces).is_some() {
                return Some(next);
            }
        }
        None
    }
}

/// The spread toward the first shuffled direction that can hold one.
pub fn spread(config: Config, ctx: &mut Ctx, at: [i32; 3], state: Block, from: usize) {
    // 1403747f0 starts from up, down, north, south, west, east.
    let mut toward = spreader::SPREAD_ORDER;
    shuffle(&mut toward, thread(ctx));
    super::trace::line(|| {
        let found = toward.iter().map(|&to| match spreader::spread_target(config, ctx.view, state, at, from, to) {
            Some((c, face, _)) => {
                let on = step(c, face);
                format!("{to}:{},{},{},{face} on {:?} in {:?}", c[0], c[1], c[2], ctx.view.block(on[0], on[1], on[2]), ctx.view.block(c[0], c[1], c[2]))
            }
            None => format!("{to}:-"),
        });
        format!("    spread {} {} {} from {from} toward {}", at[0], at[1], at[2], found.collect::<Vec<_>>().join(" "))
    });
    if let Some((cell, _, block)) = toward.iter().find_map(|&to| spreader::spread_target(config, ctx.view, state, at, from, to)) {
        ctx.view.set_block(cell[0], cell[1], cell[2], block);
    }
}
