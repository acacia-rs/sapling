//! Placing one feature (`IFeature::place` with a placement context): the wrapper features that
//! place others, each child at its parent's position and with its parent's generator.

use super::data::FEATURES;
use super::molang::{Molang, Scope, World};
use super::scatter::{self, Draws};
use super::spec::{Axis, EarlyOut, Feature, Scatter, Search};
use super::view::View;
use crate::random::Mt19937;

/// What `IFeature::PlacementContext` carries: the world, the placing generator, the Molang scope
/// and the biome at a position (None outside the world's height).
pub struct Ctx<'c, 'v> {
    pub view: &'c mut View<'v>,
    pub random: &'c mut Mt19937,
    pub scope: &'c mut Scope,
    pub biome: &'c dyn Fn([i32; 3]) -> Option<u32>,
}

impl Feature {
    /// Where the feature placed, if it did.
    pub fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        let placed = |done: bool| done.then_some(at);
        match self {
            Feature::Ore(ore) => placed(ore.place(ctx.view, at, ctx.random)),
            Feature::Disk(disk) => placed(disk.place(ctx.view, at, ctx.random)),
            Feature::Scatter(scatter) => scatter.place(ctx, at),
            Feature::Aggregate(features, early_out) => aggregate(ctx, at, features, *early_out, false),
            Feature::Sequence(features, early_out) => aggregate(ctx, at, features, *early_out, true),
            Feature::WeightedRandom(features) => weighted(ctx, at, features),
            Feature::SingleBlock(single) => single.place(ctx, at),
            Feature::Kelp { count } => placed(super::aquatic::kelp(ctx, at, *count)),
            Feature::Seagrass { count, tall } => placed(super::aquatic::seagrass(ctx, at, *count, *tall)),
            Feature::Legacy(legacy) => placed(legacy.place(ctx, at)),
            Feature::Tree(tree) => tree.place(ctx, at),
            Feature::Search(search) => search.place(ctx, at),
            Feature::LegacyTree(legacy) => legacy.place(ctx, at),
            Feature::Geode(geode) => geode.place(ctx, at),
        }
    }
}

impl Search {
    /// `SearchFeature::place` (143ba64d0): the feature at each cell of the box, loops in BDS's
    /// order for the axis, until `required` succeed (the last success is returned).
    fn place(&self, ctx: &mut Ctx, at: [i32; 3]) -> Option<[i32; 3]> {
        let mut successes = 0;
        let values = |(_, sign, [lo, hi]): (Axis, i32, [i32; 2])| -> Vec<i32> {
            if sign > 0 { (lo..=hi).collect() } else { (lo..=hi).rev().collect() }
        };
        let slot = |axis: Axis| match axis {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        };
        let [outer, middle, inner] = self.loops;
        for a in values(outer) {
            for b in values(middle) {
                for c in values(inner) {
                    let mut p = at;
                    p[slot(outer.0)] += a;
                    p[slot(middle.0)] += b;
                    p[slot(inner.0)] += c;
                    if let Some(done) = FEATURES[self.feature].place(ctx, p) {
                        successes += 1;
                        if successes == self.required {
                            return Some(done);
                        }
                    }
                }
            }
        }
        None
    }
}

impl Scatter {
    /// `ScatterFeature::place` (143b9b5d0): optionally down to the first non-air block, then the
    /// distribution around the position, every draw from the placing generator. The last success wins.
    fn place(&self, ctx: &mut Ctx, mut at: [i32; 3]) -> Option<[i32; 3]> {
        if self.project_to_floor {
            while at[1] > crate::terrain::MIN_Y && ctx.view.block(at[0], at[1] - 1, at[2]) == crate::block::Block::Air {
                at[1] -= 1;
            }
        }
        let saved = ctx.scope.save();
        let saved_biome = std::mem::replace(&mut ctx.scope.biome, (ctx.biome)(at));
        ctx.scope.set_origin(at);
        let queries = ctx.scope.queries;
        let count = {
            let host = World { view: ctx.view, queries };
            let draws = &mut Draws { positions: ctx.random, molang: None };
            scatter::iterations(&self.distribution, draws, &mut Molang { scope: ctx.scope, host: &host })
        };
        let mut placed = None;
        for index in (0..count).rev() {
            let position = {
                let host = World { view: ctx.view, queries };
                let draws = &mut Draws { positions: ctx.random, molang: None };
                scatter::position(&self.distribution, index, at, draws, &mut Molang { scope: ctx.scope, host: &host })
            };
            placed = FEATURES[self.feature].place(ctx, position).or(placed);
        }
        ctx.scope.restore(saved);
        ctx.scope.biome = saved_biome;
        placed
    }
}

/// `AggregateFeature<0/1>::place` (144c1b820, 144c1ba00). `first_failure` stops only while nothing
/// has succeeded yet, as BDS tests the accumulated result.
fn aggregate(ctx: &mut Ctx, at: [i32; 3], features: &[usize], early_out: EarlyOut, sequence: bool) -> Option<[i32; 3]> {
    let mut placed = None;
    for &feature in features {
        let position = if sequence { placed.unwrap_or(at) } else { at };
        if let Some(done) = FEATURES[feature].place(ctx, position) {
            placed = Some(done);
            if early_out == EarlyOut::FirstSuccess {
                return placed;
            }
        }
        if early_out == EarlyOut::FirstFailure && placed.is_none() {
            break;
        }
    }
    placed
}

/// `WeightedRandomFeature::place` (143bcfb10): `nextInt` of the summed weights (truncated after each
/// float add), then the first feature the remainder falls in.
fn weighted(ctx: &mut Ctx, at: [i32; 3], features: &[(usize, f32)]) -> Option<[i32; 3]> {
    let total = features.iter().fold(0i32, |total, &(_, weight)| (total as f32 + weight) as i32);
    let mut left = if total == 0 { 0.0 } else { ctx.random.next_int(total) as f32 };
    for &(feature, weight) in features {
        left -= weight;
        if left < 0.0 {
            return FEATURES[feature].place(ctx, at);
        }
    }
    None
}
