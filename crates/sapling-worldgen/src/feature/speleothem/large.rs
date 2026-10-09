//! `LargeSpeleothemFeature::place` (149e94510): Java 1.18's large dripstone with Bedrock's column
//! radius 3..18, moved back by 149e94ab0 and built by 149e94de0.

use super::cluster::{Column, scan};
use super::{SpeleothemBlocks, empty_or_water, get};
use crate::block::Block;
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::mth;
use crate::random::Mt19937;
use crate::terrain::{HEIGHT, MIN_Y};

const SEARCH_RANGE: i32 = 30;
const RADIUS: [i32; 2] = [3, 18];
const RADIUS_TO_HEIGHT: f32 = 0.33;
const STALACTITE_BLUNTNESS: [f32; 2] = [0.3, 0.9];
const STALAGMITE_BLUNTNESS: [f32; 2] = [0.4, 1.0];
const HEIGHT_SCALE: [f32; 2] = [0.4, 2.0];
const WIND_SPEED: [f32; 2] = [0.0, 0.3];
const MIN_RADIUS_FOR_WIND: i32 = 4;
const MIN_BLUNTNESS_FOR_WIND: f32 = 0.6;

fn uniform(random: &mut Mt19937, [min, max]: [f32; 2]) -> f32 {
    (max - min) * random.next_float() + min
}

/// `getDripstoneHeight` (14829e6d0), in f64 between f32 ends.
fn dripstone_height(distance: f32, radius: i32, scale: f32, bluntness: f32) -> f32 {
    let r = (bluntness.max(distance) / radius as f32) as f64 * 0.384;
    let shape = r.powf(1.333_333_333_333_333_3) * 0.75 - r.powf(0.666_666_666_666_666_6) + r.ln() * -0.333_333_333_333_333_3;
    ((shape * scale as f64).max(0.0) / 0.384 * radius as f64) as f32
}

/// Air, water or lava in the block or its second layer.
fn open(block: Block) -> bool {
    empty_or_water(block) || block.is_lava()
}

/// The wind's offset of a cell: `(origin y - y) * speed`, truncated, per horizontal axis.
#[derive(Clone, Copy)]
struct Wind(Option<(i32, f32, f32)>);

impl Wind {
    fn at(self, [x, y, z]: [i32; 3]) -> [i32; 3] {
        match self.0 {
            Some((origin, wx, wz)) => {
                let dy = (origin - y) as f32;
                [x + (dy * wx) as i32, y, z + (dy * wz) as i32]
            }
            None => [x, y, z],
        }
    }
}

struct Large {
    root: [i32; 3],
    up: bool,
    radius: i32,
    bluntness: f32,
    scale: f32,
}

impl Large {
    fn height(&self) -> i32 {
        dripstone_height(0.0, self.radius, self.scale, self.bluntness) as i32
    }

    /// `isCircleMostlyEmbeddedInStone` (14829dfb0).
    fn embedded(view: &View, [x, y, z]: [i32; 3], radius: i32) -> bool {
        if open(view.block(x, y, z)) {
            return false;
        }
        let r = radius as f32;
        let mut angle = 0.0f32;
        while angle < 6.283_185_5 {
            let (dx, dz) = ((mth::cos(angle) * r) as i32, (mth::sin(angle) * r) as i32);
            if open(view.block(x + dx, y, z + dz)) {
                return false;
            }
            angle += 6.0 / r;
        }
        true
    }

    /// `moveBackUntilBaseOrLava` (149e94ab0): up to 10 cells into the rock, halving the radius.
    fn move_back(&mut self, view: &View, wind: Wind) -> bool {
        while self.radius > 1 {
            let steps = self.height().min(10);
            for i in 0..steps {
                let p = [self.root[0], self.root[1] + if self.up { -i } else { i }, self.root[2]];
                if get(view, p).is_lava() {
                    return false;
                }
                if Self::embedded(view, wind.at(p), self.radius) {
                    self.root = p;
                    return true;
                }
            }
            self.radius /= 2;
        }
        false
    }

    /// `placeBlocks` (149e94de0): per column of the disc, base blocks into open cells until the first
    /// closed one past them; spikes are replaced, stalagmites stop at the pre-worldgen heightmap.
    fn place(&self, ctx: &mut Ctx, blocks: &SpeleothemBlocks, wind: Wind) {
        let r = self.radius;
        for dx in -r..=r {
            for dz in -r..=r {
                let distance = ((dx * dx + dz * dz) as f32).sqrt();
                if distance > r as f32 {
                    continue;
                }
                let mut h = dripstone_height(distance, r, self.scale, self.bluntness) as i32;
                if h <= 0 {
                    continue;
                }
                if ctx.random.next_float() < 0.2 {
                    h = ((ctx.random.next_float() * 0.199_999_99 + 0.8) * h as f32) as i32;
                }
                let (x, z) = (self.root[0] + dx, self.root[2] + dz);
                let top = if self.up { ctx.view.generated_top(x, z) } else { i32::MAX };
                let mut y = self.root[1];
                let mut placed = false;
                for _ in 0..h {
                    if y >= top {
                        break;
                    }
                    let p = wind.at([x, y, z]);
                    let block = get(ctx.view, p);
                    if open(block) || is_spike(block) {
                        ctx.view.set_block(p[0], p[1], p[2], blocks.base);
                        placed = true;
                    } else if placed {
                        break;
                    }
                    y += if self.up { 1 } else { -1 };
                }
            }
        }
    }
}

fn is_spike(block: Block) -> bool {
    let within = |first: Block| (first..=Block::ALL[first.index() + 9]).contains(&block.dry());
    within(Block::PointedDripstone) || within(Block::SulfurSpike)
}

pub fn place(ctx: &mut Ctx, blocks: &SpeleothemBlocks, at: [i32; 3]) -> Option<[i32; 3]> {
    crate::feature::trace::line(|| format!("  large {} {} {} draw {}", at[0], at[1], at[2], ctx.random.draws()));
    if !empty_or_water(get(ctx.view, at)) {
        return None;
    }
    let edge = |b: Block| blocks.is_base(b) || b.is_lava();
    let Some(Column { floor: Some(floor), ceiling: Some(ceiling) }) = scan(ctx.view, at, SEARCH_RANGE, empty_or_water, edge) else {
        return None;
    };
    if ceiling - floor < 5 {
        return None;
    }
    let cap = (((ceiling - floor - 1) as f32 * RADIUS_TO_HEIGHT) as i32).clamp(RADIUS[0], RADIUS[1]);
    let radius = RADIUS[0] + ctx.random.next_int(cap - RADIUS[0] + 1);
    let random = &mut *ctx.random;
    let (blunt_up, scale_up) = (uniform(random, STALACTITE_BLUNTNESS), uniform(random, HEIGHT_SCALE));
    let (blunt_down, scale_down) = (uniform(random, STALAGMITE_BLUNTNESS), uniform(random, HEIGHT_SCALE));
    let mut stalactite = Large { root: [at[0], ceiling - 1, at[2]], up: false, radius, bluntness: blunt_up, scale: scale_up };
    let mut stalagmite = Large { root: [at[0], floor + 1, at[2]], up: true, radius, bluntness: blunt_down, scale: scale_down };
    let mut wind = Wind(None);
    if radius >= MIN_RADIUS_FOR_WIND && blunt_up >= MIN_BLUNTNESS_FOR_WIND && blunt_down >= MIN_BLUNTNESS_FOR_WIND {
        let speed = uniform(random, WIND_SPEED);
        let angle = random.next_float() * std::f32::consts::PI + 0.0;
        wind = Wind(Some((at[1], mth::cos(angle) * speed, mth::sin(angle) * speed)));
    }
    let hangs = stalactite.move_back(ctx.view, wind);
    let stands = stalagmite.move_back(ctx.view, wind);
    let (min, max) = (MIN_Y, MIN_Y + HEIGHT as i32);
    let mut placed = false;
    let tip = |large: &Large| dripstone_height(0.0, large.radius, large.scale, large.bluntness);
    if hangs && ((stalactite.root[1] as f32 - tip(&stalactite)) as i32) > min {
        stalactite.place(ctx, blocks, wind);
        placed = true;
    }
    if stands && ((stalagmite.root[1] as f32 + tip(&stalagmite)) as i32) < max {
        stalagmite.place(ctx, blocks, wind);
        placed = true;
    }
    placed.then_some(at)
}
