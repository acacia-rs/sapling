//! `minecraft:geode_feature` (`GeodeFeature::place` 149e845f0, row scan 149e867f0): Java 1.17's geode
//! in f32 on the placing MT, with BDS's exclusive ranges, its own scan order, cracks set at once and
//! the layers buffered until the scan ends.

use super::place::Ctx;
use super::spec::Geode;
use super::view::View;
use crate::block::Block;
use crate::noise::NormalNoise;

const INNER: f32 = 2.2;
const MIDDLE: f32 = 3.2;
const OUTER: f32 = 4.2;
/// `1 / sqrt(1.7)`, the filling threshold, as BDS stores it.
const FILLING_BITS: u32 = 0x3f44_57d1;
/// BDS hardcodes these instead of reading the JSON's.
const CRACK_CHANCE: f32 = 0.95;
const BASE_CRACK: f32 = 2.0;
/// The faces a bud is tried on (14a7445a0), as FacingID, and each one's step.
const FACES: [usize; 6] = [1, 0, 2, 3, 4, 5];
const STEPS: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

/// `min + nextInt(max - min)`, no draw when max <= min: the maximum is never reached.
fn sample(random: &mut crate::random::Mt19937, [min, max]: [i32; 2]) -> i32 {
    if max > min { min + random.next_int(max - min) } else { min }
}

/// The float `0x5f3759df` inverse square root with one Newton step.
fn inv_sqrt(x: f32) -> f32 {
    let y = f32::from_bits(0x5f37_59df - (x.to_bits() >> 1));
    (x * -0.5 * y * y + 1.5) * y
}

fn dist_sq(a: [i32; 3], b: [i32; 3]) -> i32 {
    let d = |i: usize| (a[i] - b[i]) * (a[i] - b[i]);
    d(2) + d(1) + d(0)
}

/// Air, liquids, ice and bedrock cannot anchor a distribution point (149e84e3d).
fn invalid_anchor(block: Block) -> bool {
    matches!(block, Block::Air | Block::Water | Block::Lava | Block::Ice | Block::PackedIce | Block::Bedrock) || block.is_waterlogged()
}

/// The world-gen setter refuses `features_cannot_replace` blocks.
fn set(view: &mut View, [x, y, z]: [i32; 3], block: Block) -> bool {
    view.block(x, y, z) != Block::Bedrock && view.set_block(x, y, z, block)
}

struct Scan<'g> {
    geode: &'g Geode,
    noise: NormalNoise,
    origin: [i32; 3],
    points: Vec<([i32; 3], i32)>,
    cracks: Vec<[i32; 3]>,
    crack: bool,
    /// 1/sqrt of crack size and of the inner, middle and outer layer sizes.
    crack_threshold: f32,
    layers: [f32; 3],
    buffered: Vec<([i32; 3], Block)>,
    potential: Vec<[i32; 3]>,
    placed: Option<[i32; 3]>,
}

impl Scan<'_> {
    /// One row of z at (x, y); whether the y walk stops here: a row with no geode after rows with some,
    /// counting rows only once something was placed (a crack).
    fn row(&mut self, ctx: &mut Ctx, x: i32, y: i32, found: &mut bool, flag: &mut bool) -> bool {
        let r = self.geode.max_radius;
        let mut placed_row = false;
        for z in self.origin[2] - r..=self.origin[2] + r {
            if self.cell(ctx, [x, y, z]) && self.placed.is_some() {
                *flag = true;
                placed_row = true;
            }
        }
        if placed_row || *flag {
            *found = true;
        }
        !placed_row && *flag
    }

    /// Whether the cell is inside the outer layer.
    fn cell(&mut self, ctx: &mut Ctx, at: [i32; 3]) -> bool {
        let g = self.geode;
        let t = self.noise.sample(at[0] as f32, at[1] as f32, at[2] as f32) * g.noise_multiplier;
        let sum = self.points.iter().fold(0.0f32, |s, &(p, offset)| s + inv_sqrt((offset + dist_sq(at, p)) as f32));
        let u = self.points.len() as f32 * t + sum;
        let [inner, middle, outer] = self.layers;
        if u < outer {
            return false;
        }
        let v = self.cracks.iter().fold(0.0f32, |s, &c| s + inv_sqrt(dist_sq(at, c) as f32 + g.crack_point_offset));
        let filling = f32::from_bits(FILLING_BITS);
        if self.crack && v + t * self.cracks.len() as f32 >= self.crack_threshold && u < filling {
            if set(ctx.view, at, Block::Air) {
                self.placed = Some(at);
            }
            return true;
        }
        if u >= filling {
            self.buffered.push((at, g.filler));
        } else if u >= inner {
            if g.alternate_chance <= ctx.random.next_float() {
                self.buffered.push((at, g.inner));
                if g.placements_require_alternate {
                    return true;
                }
            } else {
                self.buffered.push((at, g.alternate_inner));
            }
            if ctx.random.next_float() < g.potential_placements_chance {
                self.potential.push(at);
            }
        } else if u >= middle {
            self.buffered.push((at, g.middle));
        } else {
            self.buffered.push((at, g.outer));
        }
        true
    }

    /// x outward from the origin, +x then -x, each x walking y up from the origin then down.
    fn run(&mut self, ctx: &mut Ctx) {
        let ([ox, oy, _], r) = (self.origin, self.geode.max_radius);
        let mut any = false;
        let slice = |scan: &mut Self, ctx: &mut Ctx, x: i32| {
            let (mut found, mut flag) = (false, false);
            let mut y = oy;
            while y <= oy + r && !scan.row(ctx, x, y, &mut found, &mut flag) {
                y += 1;
            }
            let up = flag;
            flag = false;
            let mut y = oy;
            while y > oy - r {
                y -= 1;
                if scan.row(ctx, x, y, &mut found, &mut flag) {
                    break;
                }
            }
            (found, up || flag)
        };
        for x in ox..=ox + r {
            let (found, some) = slice(self, ctx, x);
            any |= some;
            if !found && any {
                break;
            }
        }
        for x in (ox - r..ox).rev() {
            let (found, some) = slice(self, ctx, x);
            if !found && (any || some) {
                break;
            }
            any |= some;
        }
    }
}

impl Geode {
    /// Not in a structure (149e845f0 asks the dimension's generator first, before any draw).
    pub fn place(&self, ctx: &mut Ctx, origin: [i32; 3]) -> Option<[i32; 3]> {
        if (ctx.in_structure)(origin) {
            return None;
        }
        let count = sample(ctx.random, self.points);
        let noise = NormalNoise::from_mt(ctx.random, -4, &[1.0]);
        let d = count as f32 / self.wall[1] as f32;
        let layers = [INNER, MIDDLE, OUTER].map(|size| 1.0 / (size + d).sqrt());
        let crack_size = if count >= 4 { d } else { 0.0 } + (ctx.random.next_float() * 0.5 + BASE_CRACK);
        let crack_threshold = 1.0 / crack_size.sqrt();
        let crack = ctx.random.next_float() < CRACK_CHANCE;
        let (mut points, mut invalid) = (Vec::new(), 0);
        for _ in 0..count {
            let offset = [0; 3].map(|_| sample(ctx.random, self.wall));
            let at = [0, 1, 2].map(|i| origin[i] + offset[i]);
            if invalid_anchor(ctx.view.block(at[0], at[1], at[2])) {
                if invalid >= self.invalid_blocks_threshold {
                    return None;
                }
                invalid += 1;
            }
            points.push((at, sample(ctx.random, self.point_offset)));
        }
        let mut cracks = Vec::new();
        if crack {
            let s = count * 2 + 1;
            let (dx, dz) = match ctx.random.next_int(4) {
                0 => (s, 0),
                1 => (0, s),
                2 => (s, s),
                _ => (0, 0),
            };
            cracks.extend([7, 5, 1].map(|dy| [origin[0] + dx, origin[1] + dy, origin[2] + dz]));
        }
        let mut scan = Scan { geode: self, noise, origin, points, cracks, crack, crack_threshold, layers, buffered: Vec::new(), potential: Vec::new(), placed: None };
        scan.run(ctx);
        for &(at, block) in &scan.buffered {
            if set(ctx.view, at, block) {
                scan.placed = Some(at);
            }
        }
        if !self.placements.is_empty() {
            for &at in &scan.potential {
                let bud = self.placements[ctx.random.next_int(self.placements.len() as i32) as usize];
                if let Some(target) = place_bud(ctx.view, at, bud) {
                    scan.placed = Some(target);
                }
            }
        }
        scan.placed
    }
}

/// The first face whose neighbour is air or water and holds the bud on `at`.
fn place_bud(view: &mut View, at: [i32; 3], bud: Block) -> Option<[i32; 3]> {
    for face in FACES {
        let target = [0, 1, 2].map(|i| at[i] + STEPS[face][i]);
        let there = view.block(target[0], target[1], target[2]);
        if matches!(there, Block::Air | Block::Water) && view.block(at[0], at[1], at[2]).is_solid() {
            return set(view, target, bud.faced(face)).then_some(target);
        }
    }
    None
}
