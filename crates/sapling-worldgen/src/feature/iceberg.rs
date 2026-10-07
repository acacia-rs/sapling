//! `minecraft:legacy:ice_feature` (149e96790): in 1 of 8 frozen-ocean chunks an iceberg at sea level,
//! `IcebergFeature::place` (149e8f460, cut-out carve 149e90f80): Java 1.17's in f32 with BDS's draw
//! order, a 1% blue-ice body and snow on the top part; then up to 19 blue ice patches.

use std::f32::consts::{FRAC_PI_2, PI};

use super::view::View;
use crate::block::Block;
use crate::mth;
use crate::random::Mt19937;

const SEA_LEVEL: i32 = 63;

fn is_iceberg(block: Block) -> bool {
    matches!(block, Block::PackedIce | Block::Snow | Block::BlueIce)
}

/// `(int)f` plus one when f has a fraction above it.
fn ceil(f: f32) -> i32 {
    let i = f as i32;
    i + ((i as f32) < f) as i32
}

pub fn ice_feature(view: &mut View, random: &mut Mt19937, [x, _, z]: [i32; 3]) -> bool {
    if random.next_int(8) == 0 {
        let dx = random.next_int(8);
        let dz = random.next_int(8);
        Iceberg::new(random).place(view, random, [x + dx + 12, SEA_LEVEL, z + dz + 12]);
    }
    for _ in 0..random.next_int(20) {
        let dx = random.next_int(16);
        let y = random.next_int(32) + 30;
        let dz = random.next_int(16);
        blue_ice(view, random, [x + dx + 8, y, z + dz + 8]);
    }
    true
}

const SIDES: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

fn at(view: &View, [x, y, z]: [i32; 3], [dx, dy, dz]: [i32; 3]) -> Block {
    view.block(x + dx, y + dy, z + dz)
}

/// `BlueIceFeature` (vtable slot 4, 14827ec60): under sea level, in or over water, beside packed ice
/// (not below); then 200 tries grow blue ice next to 1..4 blue ice blocks.
fn blue_ice(view: &mut View, random: &mut Mt19937, origin: [i32; 3]) {
    if origin[1] >= SEA_LEVEL || (at(view, origin, [0; 3]) != Block::Water && at(view, origin, SIDES[0]) != Block::Water) {
        return;
    }
    if !SIDES[1..].iter().any(|&side| at(view, origin, side) == Block::PackedIce) {
        return;
    }
    view.set_block(origin[0], origin[1], origin[2], Block::BlueIce);
    for _ in 0..200 {
        let dy = random.next_int(5) - random.next_int(6);
        if dy <= -6 {
            continue;
        }
        let k = if dy < 2 { dy / 2 + 3 } else { 3 };
        let (a, b, c, d) = (random.next_int(k), random.next_int(k), random.next_int(k), random.next_int(k));
        let pos = [origin[0] + a - b, origin[1] + dy, origin[2] + c - d];
        if !matches!(at(view, pos, [0; 3]), Block::Air | Block::Water | Block::PackedIce | Block::Ice) {
            continue;
        }
        let blue = SIDES.iter().filter(|&&side| at(view, pos, side) == Block::BlueIce).count();
        if (1..=4).contains(&blue) {
            view.set_block(pos[0], pos[1], pos[2], Block::BlueIce);
        }
    }
}

struct Iceberg {
    /// The ellipse's long radius, 7..11.
    a: i32,
    /// The ellipse's short radius, 3..5.
    shift: i32,
    ellipse: bool,
    /// The square the top part spans, and its height.
    o: i32,
    height: i32,
    snow: bool,
    angle: f32,
    radius: i32,
    body: Block,
}

impl Iceberg {
    fn new(random: &mut Mt19937) -> Self {
        let a = 11 - random.next_int(5);
        let shift = random.next_int(3) + 3;
        let tall = random.next_int(6);
        let low = random.next_int(15);
        let ellipse = random.next_float() > 0.8;
        let (o, height) = match ellipse {
            true => (a, tall + 6),
            false if random.next_float() <= 0.9 => (11, low + 3),
            false => (11, low + random.next_int(19) + 10),
        };
        let snow = random.next_float() > 0.7;
        let turn = random.next_float();
        let angle = (turn + turn) * PI;
        let grow = random.next_int(7);
        let radius = (grow + height - random.next_int(5)).min(11);
        let body = if random.next_float() <= 0.01 { Block::BlueIce } else { Block::PackedIce };
        Self { a, shift, ellipse, o, height, snow, angle, radius, body }
    }

    fn place(&self, view: &mut View, random: &mut Mt19937, origin: [i32; 3]) {
        let (o, height) = (self.o, self.height);
        if o > 0 && height > 0 {
            for p in -o..o {
                for q in -o..o {
                    for y in 0..height {
                        let s = if self.ellipse { o } else { self.radius_round(random, y, height) };
                        if self.ellipse || p < s {
                            self.block(view, random, origin, [p, y, q], height - y, height, s, o);
                        }
                    }
                }
            }
        }
        self.smooth(view, origin);
        let depth = random.next_int(11) + height;
        let deep = depth.min(18);
        if o > 0 && depth > 1 {
            for p in -o..o {
                for q in -o..o {
                    let (mut y, mut down) = (-1, 1);
                    while y > -deep {
                        let reach = if self.ellipse { ceil((1.0 - (y * y) as f32 / (deep as f32 * 8.0)) * o as f32) } else { o };
                        let s = self.radius_steep(random, down, deep);
                        if p < s {
                            self.block(view, random, origin, [p, y, q], deep + down, deep, s, reach);
                        }
                        y -= 1;
                        down += 1;
                    }
                }
            }
        }
        self.cut_out(view, random, origin);
    }

    /// `heightDependentRadiusRound`, drawn per call.
    fn radius_round(&self, random: &mut Mt19937, y: i32, height: i32) -> i32 {
        let f = (3.5 - random.next_float()) * height as f32;
        let g = if random.next_int(5) + 15 < height {
            let below = random.next_int(6) + 3;
            let i = if y < below { y >> 1 } else { y };
            (i as f32 / (f * -0.4) + 1.0) * self.radius as f32
        } else {
            let yf = y as f32;
            (1.0 - (yf * yf) / f) * self.radius as f32
        };
        ceil(g * 0.5)
    }

    /// `heightDependentRadiusSteep` for `down` blocks below sea level.
    fn radius_steep(&self, random: &mut Mt19937, down: i32, height: i32) -> i32 {
        let f = random.next_float();
        ceil((1.0 - down as f32 / ((f * 0.5 + 1.0) * height as f32)) * self.radius as f32 * 0.5)
    }

    /// The rotated ellipse's signed distance (cos, sin from the Mth table).
    fn ellipse_distance(&self, angle: f32, dx: f32, dz: f32, a: f32, c: f32) -> f32 {
        let (cos, sin) = (mth::cos(angle), mth::sin(angle));
        let u = (cos * dx - sin * dz) / a;
        let v = (cos * dz + sin * dx) / c;
        v * v + u * u + -1.0
    }

    /// `generateIcebergBlock` and `setIcebergBlock` at `origin + [p, y, q]`.
    #[allow(clippy::too_many_arguments)]
    fn block(&self, view: &mut View, random: &mut Mt19937, origin: [i32; 3], [p, y, q]: [i32; 3], left: i32, height: i32, s: i32, reach: i32) {
        let (distance, edge) = if self.ellipse {
            (self.ellipse_distance(self.angle, p as f32, q as f32, reach as f32, self.shift as f32), None)
        } else {
            let f = random.next_float();
            let c = if f > 0.8 { 8.0 } else { f.max(0.2) * 10.0 };
            let square = ((p * p + q * q) as f64 - (s as f64) * (s as f64)) as f32;
            (square + c / s as f32, Some(()))
        };
        if !(distance < 0.0) {
            return;
        }
        let at = [origin[0] + p, origin[1] + y, origin[2] + q];
        let threshold = if edge.is_some() { -6.0 - random.next_int(3) as f32 } else { -0.5 };
        if threshold < distance && random.next_float() > 0.9 {
            return;
        }
        let here = view.block(at[0], at[1], at[2]);
        if !matches!(here, Block::Air | Block::Snow | Block::Ice | Block::Water) {
            return;
        }
        let (divisor, keep) = if self.ellipse { (3, random.next_float() <= 0.05) } else { (2, false) };
        let mut block = self.body;
        if self.snow && here != Block::Water {
            let span = height / divisor;
            if left as f32 <= (random.next_int(span + (span == 0) as i32) as f32 + height as f32 * 0.6) {
                block = Block::Snow;
            }
            if keep {
                block = self.body;
            }
        }
        view.set_block(at[0], at[1], at[2], block);
    }

    /// Ice over air falls away with what is above; ice with three open sides goes.
    fn smooth(&self, view: &mut View, origin: [i32; 3]) {
        let i = if self.ellipse { self.a } else { self.radius / 2 };
        for x in -i..=i {
            for z in -i..=i {
                for y in 0..=self.height {
                    let [px, py, pz] = [origin[0] + x, origin[1] + y, origin[2] + z];
                    let here = view.block(px, py, pz);
                    if !is_iceberg(here) && here != Block::SnowLayer {
                        continue;
                    }
                    if view.block(px, py - 1, pz) == Block::Air {
                        view.set_block(px, py, pz, Block::Air);
                        view.set_block(px, py + 1, pz, Block::Air);
                    } else if is_iceberg(here) {
                        let open = [[0, 1], [-1, 0], [0, -1], [1, 0]].iter().filter(|[dx, dz]| !is_iceberg(view.block(px + dx, py, pz + dz))).count();
                        if open > 2 {
                            view.set_block(px, py, pz, Block::Air);
                        }
                    }
                }
            }
        }
    }

    fn cut_out(&self, view: &mut View, random: &mut Mt19937, origin: [i32; 3]) {
        let first = random.next_float();
        let go = if self.ellipse { first > 0.1 } else { random.next_float() > 0.7 };
        if !go {
            return;
        }
        let r = self.radius;
        let offset = |random: &mut Mt19937| {
            let k = random.next_int((r / 2).max(3) - 2);
            if random.next_boolean() { r / 2 - random.next_int((r - r / 2 - 1).max(1)) + 1 } else { k }
        };
        let (mut k, mut l) = (offset(random), offset(random));
        if self.ellipse {
            k = random.next_int((self.a - 5).max(1));
            l = k;
        }
        let kx = if random.next_boolean() { -k } else { k };
        let lz = if random.next_boolean() { -l } else { l };
        let angle = if self.ellipse { self.angle + FRAC_PI_2 } else { (|f: f32| (f + f) * PI)(random.next_float()) };
        let centre = [kx, lz];
        if self.height > 3 {
            for y in 0..self.height - 3 {
                let radius = self.radius_round(random, y, self.height);
                self.carve(view, origin, radius, y, false, angle, centre);
            }
        }
        let (mut y, mut down) = (-1, 1);
        while random.next_int(5) - self.height < y {
            let f = random.next_float();
            let radius = ceil((1.0 - down as f32 / ((f * 0.5 + 1.0) * self.height as f32)) * self.radius as f32 * 0.5);
            self.carve(view, origin, radius, y, true, angle, centre);
            y -= 1;
            down += 1;
        }
    }

    /// 149e90f80: empties (or floods, below sea level) the iceberg inside an ellipse.
    #[allow(clippy::too_many_arguments)]
    fn carve(&self, view: &mut View, origin: [i32; 3], radius: i32, y: i32, water: bool, angle: f32, [cx, cz]: [i32; 2]) {
        let i = radius + self.a / 3 + 1;
        let j = radius.min(6) + self.shift / 2 - 4;
        for k in -i..i {
            for l in -i..i {
                if !(self.ellipse_distance(angle, (k - cx) as f32, (l - cz) as f32, i as f32, j as f32) < 0.0) {
                    continue;
                }
                let [px, py, pz] = [origin[0] + k, origin[1] + y, origin[2] + l];
                let here = view.block(px, py, pz);
                if !is_iceberg(here) && here != Block::SnowLayer {
                    continue;
                }
                if water {
                    view.set_block(px, py, pz, Block::Water);
                } else {
                    view.set_block(px, py, pz, Block::Air);
                    if view.block(px, py + 1, pz) == Block::SnowLayer {
                        view.set_block(px, py + 1, pz, Block::Air);
                    }
                }
            }
        }
    }
}
