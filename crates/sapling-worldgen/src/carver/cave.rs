//! The data-driven cave carver (`minecraft:cave_carver_feature`), as BDS builds its tunnel
//! segments for one source chunk: `addFeature` 148284810, `addRoom` 1482830a0, `addTunnel`
//! 148283530, with the 1.18 variant functions of config 14a925940.

use std::f32::consts::{FRAC_PI_2, PI};

use crate::random::{Mt19937, unit};

/// One ellipsoid of a tunnel or room, in BDS's 0-based local y (block y + 64).
#[derive(Debug, Clone, Copy)]
pub struct Segment {
    pub center: [f32; 3],
    pub horizontal_radius: f32,
    pub vertical_radius: f32,
    pub floor_level: f32,
    pub step: i32,
    pub length: i32,
    pub thickness: f32,
}

/// A `minecraft:cave_carver_feature` JSON plus the constructor defaults (1482817d0) it does not
/// set. Only the extra carver has a vanilla feature rule, yet BDS runs both (oracle-confirmed).
pub struct CaveSettings {
    skip_carve_chance: i32,
    pub height_limit: i32,
    y_scale: (f32, f32),
    horizontal_radius_multiplier: (f32, f32),
    vertical_radius_multiplier: (f32, f32),
    floor_level: (f32, f32),
    width_modifier: f32,
    cave_bound: i32,
    room_chance: i32,
    extra_tunnels: i32,
    skip_segment_chance: i32,
    steep_chance: i32,
}

/// `minecraft:underground_extra_cave_carver_feature`.
pub const EXTRA_CAVE_CARVER: CaveSettings = CaveSettings { skip_carve_chance: 66, height_limit: 103, ..UNDERGROUND_CAVE_CARVER };

/// `minecraft:underground_cave_carver_feature`.
pub const UNDERGROUND_CAVE_CARVER: CaveSettings = CaveSettings {
    skip_carve_chance: 33,
    height_limit: 236,
    y_scale: (0.1, 0.9),
    horizontal_radius_multiplier: (0.7, 1.4),
    vertical_radius_multiplier: (0.8, 1.3),
    floor_level: (-1.0, -0.4),
    width_modifier: 0.0,
    cave_bound: 40,
    room_chance: 4,
    extra_tunnels: 4,
    skip_segment_chance: 4,
    steep_chance: 6,
};

/// Per-cave values drawn once and shared by its room and tunnels.
#[derive(Clone, Copy)]
struct Cave {
    horizontal_multiplier: f32,
    vertical_multiplier: f32,
    floor_level: f32,
}

fn in_range(random: &mut Mt19937, (min, max): (f32, f32)) -> f32 {
    if min == max { min } else { min + (max - min) * random.next_float() }
}

impl CaveSettings {
    /// Every segment the caves starting in this chunk carve; `random` is seeded per chunk.
    pub fn segments(&self, random: &mut Mt19937, chunk_x: i32, chunk_z: i32) -> Vec<Segment> {
        let bound = random.next_int(self.cave_bound);
        let bound = random.next_int(bound + 1);
        let count = random.next_int(bound + 1);
        let mut out = Vec::new();
        if count == 0 || random.next_int(self.skip_carve_chance) != 0 {
            return out;
        }
        for _ in 0..count {
            // MSVC evaluates the position's arguments right to left: z, y, x.
            let z = random.next_int(16) + chunk_z * 16;
            let y = random.next_int(self.height_limit) + 8;
            let x = random.next_int(16) + chunk_x * 16;
            let origin = [x as f32, y as f32, z as f32];
            let cave = Cave {
                horizontal_multiplier: in_range(random, self.horizontal_radius_multiplier),
                vertical_multiplier: in_range(random, self.vertical_radius_multiplier),
                floor_level: in_range(random, self.floor_level),
            };
            let mut tunnels = 1;
            if random.next_int(self.room_chance) == 0 {
                self.room(random, origin, cave, &mut out);
                tunnels += random.next_int(self.extra_tunnels);
            }
            for _ in 0..tunnels {
                let yaw = random.next_float() * PI;
                let yaw = yaw + yaw;
                let pitch = random.next_float() - 0.5;
                let pitch = (pitch + pitch) * 0.125;
                let thickness = thickness(random);
                let length = 112 - random.next_int(28);
                let tunnel = Tunnel { thickness, yaw, pitch, step: 0, length, y_scale: 1.0 };
                self.tunnel(random, origin, tunnel, cave, &mut out);
            }
        }
        out
    }

    fn room(&self, parent: &mut Mt19937, origin: [f32; 3], cave: Cave, out: &mut Vec<Segment>) {
        let thickness = parent.next_float() * 6.0 + 1.0;
        let mut random = Mt19937::new(parent.next_int_unbounded() as u32);
        let y_scale = in_range(&mut random, self.y_scale);
        let length = 112 - (random.next_u32() % 28) as i32;
        let step = length >> 1;
        let radius = (step as f32 * PI / length as f32).sin() * (self.width_modifier + thickness) + 1.5;
        out.push(Segment {
            center: origin,
            horizontal_radius: radius * cave.horizontal_multiplier,
            vertical_radius: y_scale * radius * cave.vertical_multiplier,
            floor_level: cave.floor_level,
            step,
            length,
            thickness,
        });
    }

    fn tunnel(&self, parent: &mut Mt19937, mut at: [f32; 3], mut t: Tunnel, cave: Cave, out: &mut Vec<Segment>) {
        let mut random = Mt19937::new(parent.next_int_unbounded() as u32);
        let mut branch_step = 0;
        if t.length < 1 {
            t.length = 112 - (random.next_u32() % 28) as i32;
        }
        if t.length != 1 {
            branch_step = (random.next_u32() % (t.length as u32 >> 1)) as i32;
        }
        let decay = if random.next_int(self.steep_chance) != 0 { 0.7 } else { 0.92 };
        if t.step >= t.length {
            return;
        }
        branch_step += t.length >> 2;
        let (mut pitch_change, mut yaw_change) = (0.0f32, 0.0f32);
        loop {
            let sine = (t.step as f32 * PI / t.length as f32).sin();
            let width = self.width_modifier + t.thickness;
            let (cos_pitch, sin_pitch) = (t.pitch.cos(), t.pitch.sin());
            at[0] += t.yaw.cos() * cos_pitch;
            at[1] += sin_pitch;
            at[2] += t.yaw.sin() * cos_pitch;
            let decayed_pitch = t.pitch * decay;
            let next_pitch_change = 0.9 * pitch_change;
            let next_yaw_change = 0.75 * yaw_change;
            let draws: [f32; 6] = std::array::from_fn(|_| unit(random.next_u32()));
            let radius = sine * width + 1.5;
            t.pitch = pitch_change * 0.1 + decayed_pitch;
            t.yaw += yaw_change * 0.1;
            let yaw_delta = draws[5] * (draws[3] - draws[4]) * 4.0;
            if t.thickness > 1.0 && branch_step == t.step {
                for side in [-FRAC_PI_2, FRAC_PI_2] {
                    let thickness = unit(random.next_u32()) * 0.5 + 0.5;
                    let branch = Tunnel { thickness, yaw: t.yaw + side, pitch: t.pitch / 3.0, step: branch_step, length: t.length, y_scale: 1.0 };
                    self.tunnel(&mut random, at, branch, cave, out);
                }
                return;
            }
            if random.next_int(self.skip_segment_chance) != 0 {
                out.push(Segment {
                    center: at,
                    horizontal_radius: radius,
                    vertical_radius: t.y_scale * radius,
                    floor_level: cave.floor_level,
                    step: t.step,
                    length: t.length,
                    thickness: t.thickness,
                });
            }
            if t.step + 1 >= t.length {
                return;
            }
            let pitch_delta = draws[2] * (draws[0] - draws[1]);
            pitch_change = next_pitch_change + (pitch_delta + pitch_delta);
            yaw_change = next_yaw_change + yaw_delta;
            t.step += 1;
        }
    }
}

#[derive(Clone, Copy)]
struct Tunnel {
    thickness: f32,
    yaw: f32,
    pitch: f32,
    step: i32,
    length: i32,
    y_scale: f32,
}

/// 1482810b0: `2f + f`, one time in ten times `3f*f + 1`.
fn thickness(random: &mut Mt19937) -> f32 {
    let first = random.next_float();
    let doubled = first + first;
    let mut thickness = random.next_float() + doubled;
    if random.next_int(10) == 0 {
        let a = random.next_float();
        let b = random.next_float();
        thickness *= b * a * 3.0 + 1.0;
    }
    thickness
}
