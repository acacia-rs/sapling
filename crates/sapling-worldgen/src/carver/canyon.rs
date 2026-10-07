//! The hardcoded overworld canyon (`OverworldGenerator` +0x258, applied by 14827f840): its walk
//! for one source chunk (14827faa0) and its ellipsoid carve (148280880).

use std::f32::consts::PI;
use std::sync::Arc;

use crate::density::{AquiferCache, Density};
use crate::random::{Mt19937, unit};
use crate::terrain::{CellKind, ChunkShape, MIN_Y};

/// A static canyon config (14a925900/910/920, chosen in `loadChunk` by base game version). The
/// legacy ones (docs/research/worldgen-bds.md) are not ported.
pub struct CanyonSettings {
    chance: u32,
    y_offset: i32,
    max_y: i32,
    /// `+9`: one canyon in twenty may be giant.
    giant: bool,
    /// `+10`: start height `nextInt(58) + 10` instead of `nextInt(nextInt(40) + 8) + 20`.
    flat_start: bool,
}

/// 14a925920, what new worlds use; its `+8` flag also skips the legacy per-step biome test.
pub const CANYON_1_18: CanyonSettings = CanyonSettings { chance: 100, y_offset: 64, max_y: 312, giant: false, flat_start: true };

/// One carving step of a canyon, in 0-based local y.
#[derive(Debug, Clone, Copy)]
pub struct CanyonStep {
    center: [f32; 3],
    horizontal_radius: f32,
    vertical_radius: f32,
    remaining: i32,
}

#[derive(Debug)]
pub struct Canyon {
    thickness: f32,
    /// Width factor per local y (`rs`), indexed by block y - 2.
    widths: Arc<[f32; 320]>,
    steps: Vec<CanyonStep>,
}

impl CanyonSettings {
    pub fn max_y(&self) -> i32 {
        self.max_y
    }

    /// The canyon starting in this chunk, if any; `random` is seeded per chunk.
    pub fn canyon(&self, random: &mut Mt19937, chunk_x: i32, chunk_z: i32) -> Option<Canyon> {
        if random.next_bounded(self.chance) != 0 {
            return None;
        }
        let x = random.next_bounded(16) as i32 + chunk_x * 16;
        let y = if self.flat_start {
            let y = random.next_bounded(58) as i32 + self.y_offset + 10;
            random.skip(1);
            y
        } else {
            let bound = random.next_bounded(40) + 8;
            random.next_bounded(bound) as i32 + self.y_offset + 20
        };
        random.skip(1);
        let z = random.next_bounded(16) as i32 + chunk_z * 16;
        let yaw = random.next_float() * PI;
        let pitch = random.next_float() - 0.5;
        let first = random.next_float() * 3.0;
        let mut thickness = random.next_float() * 3.0 + first;
        let mut y_scale = 4.0f32;
        if random.next_float() < 0.05 && self.giant {
            thickness = thickness + thickness;
            y_scale = 40.0;
        }
        let seed = random.next_int() as u32;
        let mut walk = Mt19937::new(seed);
        let length = 120 - (walk.next_u32() % 30) as i32;
        let widths = Arc::new(widths(&mut walk, &mut Mt19937::new(seed)));
        let walk_start = Walk { at: [x as f32, y as f32, z as f32], yaw: yaw + yaw, pitch: pitch * 0.25, thickness, y_scale, length };
        Some(Canyon { thickness, widths, steps: walk_start.steps(&mut walk) })
    }
}

/// `rs`: a run-length random profile, the first 128 entries from the walk's generator and the
/// rest from a second generator with the same seed.
fn widths(first: &mut Mt19937, second: &mut Mt19937) -> [f32; 320] {
    let mut widths = [0.0; 320];
    let a = first.next_float();
    let mut value = first.next_float() * a + 1.0;
    widths[0] = value * value;
    for (i, width) in widths.iter_mut().enumerate().skip(1) {
        let random = if i < 128 { &mut *first } else { &mut *second };
        if random.next_bounded(3) == 0 {
            let a = random.next_float();
            value = random.next_float() * a + 1.0;
        }
        *width = value * value;
    }
    widths
}

struct Walk {
    at: [f32; 3],
    yaw: f32,
    pitch: f32,
    thickness: f32,
    y_scale: f32,
    length: i32,
}

impl Walk {
    fn steps(mut self, random: &mut Mt19937) -> Vec<CanyonStep> {
        let mut steps = Vec::new();
        let (mut pitch_change, mut yaw_change) = (0.0f32, 0.0f32);
        for step in 0..self.length {
            let sine = (step as f32 * PI / self.length as f32).sin();
            let base_radius = sine * self.thickness + 1.5;
            let (horizontal, vertical) = (random.next_u32(), random.next_u32());
            let (cos_pitch, sin_pitch) = (self.pitch.cos(), self.pitch.sin());
            self.at[0] += self.yaw.cos() * cos_pitch;
            self.at[1] += sin_pitch;
            self.at[2] += self.yaw.sin() * cos_pitch;
            let decayed_pitch = self.pitch * 0.7;
            let next_pitch_change = pitch_change * 0.8;
            let next_yaw_change = yaw_change * 0.5;
            let [a, b, c] = [random.next_u32(), random.next_u32(), random.next_u32()];
            let yaw_difference = unit(random.next_u32()) - unit(random.next_u32());
            let yaw_delta = unit(random.next_u32()) * yaw_difference * 4.0;
            if random.next_u32() & 3 != 0 {
                let horizontal_radius = (unit(horizontal) * 0.25 + 0.75) * base_radius;
                let vertical_radius = (unit(vertical) * 0.25 + 0.75) * (base_radius * self.y_scale);
                steps.push(CanyonStep { center: self.at, horizontal_radius, vertical_radius, remaining: self.length - step });
            }
            self.pitch = decayed_pitch + pitch_change * 0.05;
            self.yaw += yaw_change * 0.05;
            let pitch_delta = unit(c) * (unit(a) - unit(b));
            pitch_change = next_pitch_change + (pitch_delta + pitch_delta);
            yaw_change = next_yaw_change + yaw_delta;
        }
        steps
    }
}

impl Canyon {
    /// Carves the steps that reach the chunk; BDS stops the whole canyon at the first step out of
    /// range, so later steps never carve this chunk.
    pub fn carve(&self, shape: &mut ChunkShape, density: &Density, aquifer: &mut AquiferCache, chunk: (i32, i32), max_y: i32) {
        let (mid_x, mid_z) = ((chunk.0 * 16 + 8) as f32, (chunk.1 * 16 + 8) as f32);
        let reach = 2.0 + self.thickness + 16.0;
        for step in &self.steps {
            let (dx, dz) = (step.center[0] - mid_x, step.center[2] - mid_z);
            let remaining = step.remaining as f32;
            if (dz * dz + dx * dx) - remaining * remaining > reach * reach {
                return;
            }
            let margin = step.horizontal_radius + step.horizontal_radius;
            let [x, _, z] = step.center;
            if (mid_x - 16.0) - margin > x || (mid_z - 16.0) - margin > z || x > (mid_x + 16.0) + margin || z > margin + (16.0 + mid_z) {
                continue;
            }
            self.carve_step(shape, density, aquifer, step, chunk, max_y);
        }
    }

    fn carve_step(&self, shape: &mut ChunkShape, density: &Density, aquifer: &mut AquiferCache, step: &CanyonStep, chunk: (i32, i32), max_y: i32) {
        let (hr, vr) = (step.horizontal_radius, step.vertical_radius);
        let [cx, cy, cz] = step.center;
        let horizontal = |center: f32, base: i32| {
            let low = ((center - hr).floor() - base as f32 + -1.0) as i32;
            let high = ((center + hr).floor() - base as f32 + 1.0) as i32;
            (low.max(0), high.min(16))
        };
        let (x0, x1) = horizontal(cx, chunk.0 * 16);
        let (z0, z1) = horizontal(cz, chunk.1 * 16);
        let bottom = ((cy - vr).floor() + -1.0) as i32;
        let top = ((cy + vr).floor() + 1.0) as i32;
        let y1 = if max_y < top { max_y } else { top.max(1) };
        let y0 = if y1 < bottom { y1 } else { bottom.max(1) };
        for lx in x0..x1 {
            let wx = chunk.0 * 16 + lx;
            let dx = ((wx as f32 + 0.5) - cx) / hr;
            for lz in z0..z1 {
                let wz = chunk.1 * 16 + lz;
                let dz = ((wz as f32 + 0.5) - cz) / hr;
                let horizontal = dz * dz + dx * dx;
                if !(horizontal < 1.0) {
                    continue;
                }
                for local_y in (y0 + 1..=y1).rev() {
                    let dy = (((local_y - 1) as f32 + 0.5) - cy) / vr;
                    if !((dy * dy) / 6.0 + self.widths[(local_y - 2) as usize] * horizontal < 1.0) {
                        continue;
                    }
                    let y = local_y + MIN_Y;
                    let (x, z) = (lx as usize, lz as usize);
                    if shape.kind(x, y, z) != CellKind::Solid {
                        continue;
                    }
                    if let Some(kind) = super::carve::aquifer_fill(density, aquifer, [wx, y, wz]) {
                        shape.set(x, y, z, kind);
                    }
                }
            }
        }
    }
}
