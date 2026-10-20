//! The Nether's cave carver (`minecraft:nether_cave_carver_feature`, vtable 14a928470: `place` 149d577e0,
//! `addFeature` 149d589b0, `addRoom` 149d57a90, `addTunnel` 149d57b70): Java 1.12's `MapGenCavesHell` in f32 with
//! Bedrock's table sine. Each target chunk walks every tunnel of the 17x17 source chunks around it and carves
//! as it goes, as Java does: a tunnel that has left the target's reach stops for that target.

use std::f32::consts::{FRAC_PI_2, PI};

use crate::block::Block;
use crate::chunk::ChunkShape;
use crate::mth;
use crate::random::{LargeFeatureSeed, Mt19937};

const RANGE: i32 = 8;
const CAVE_BOUND: i32 = 10;
const SKIP_CARVE_CHANCE: i32 = 5;
const ROOM_CHANCE: i32 = 4;
const EXTRA_TUNNELS: i32 = 4;
const STEEP_CHANCE: i32 = 6;
const SKIP_SEGMENT_CHANCE: i32 = 4;
const FLOOR_LEVEL: f32 = -0.7;
/// The clamps of an ellipsoid's y range (immediates 2 and 0x77); `carve` tests and empties the cell above each y of
/// it and the one below, so carved cells lie in 2..=120.
const MIN_Y: i32 = 2;
const MAX_Y: i32 = 119;

pub struct NetherCarver {
    seed: LargeFeatureSeed,
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

/// The chunk being carved.
struct Target<'a> {
    shape: &'a mut ChunkShape,
    chunk: (i32, i32),
}

impl NetherCarver {
    /// The decoration MT takes the world seed's low 32 bits.
    pub fn new(world_seed: i64) -> Self {
        Self { seed: LargeFeatureSeed::new(world_seed as u32) }
    }

    pub fn carve(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        let mut target = Target { shape, chunk: (chunk_x, chunk_z) };
        for x in chunk_x - RANGE..=chunk_x + RANGE {
            for z in chunk_z - RANGE..=chunk_z + RANGE {
                add_feature(&mut Mt19937::new(self.seed.chunk(x, z)), (x, z), &mut target);
            }
        }
    }
}

fn add_feature(random: &mut Mt19937, (source_x, source_z): (i32, i32), target: &mut Target) {
    let bound = random.next_int(CAVE_BOUND);
    let bound = random.next_int(bound + 1);
    let count = random.next_int(bound + 1);
    if random.next_int(SKIP_CARVE_CHANCE) != 0 || count <= 0 {
        return;
    }
    for _ in 0..count {
        // MSVC evaluates the position's arguments right to left: z, y, x.
        let z = random.next_int(16) + source_z * 16;
        let y = random.next_int(128);
        let x = random.next_int(16) + source_x * 16;
        let origin = [x as f32, y as f32, z as f32];
        let mut tunnels = 1;
        if random.next_int(ROOM_CHANCE) == 0 {
            let thickness = random.next_float() * 6.0 + 1.0;
            let room = Tunnel { thickness, yaw: 0.0, pitch: 0.0, step: -1, length: -1, y_scale: 0.5 };
            add_tunnel(random, origin, room, target);
            tunnels = random.next_int(EXTRA_TUNNELS) + 1;
        }
        for _ in 0..tunnels {
            let yaw = random.next_float() * PI;
            let pitch = random.next_float() + -0.5;
            let first = random.next_float();
            let thickness = first + first + random.next_float();
            let tunnel = Tunnel { thickness: thickness + thickness, yaw: yaw + yaw, pitch: (pitch + pitch) * 0.125, step: 0, length: 0, y_scale: 0.5 };
            add_tunnel(random, origin, tunnel, target);
        }
    }
}

fn add_tunnel(parent: &mut Mt19937, mut at: [f32; 3], mut t: Tunnel, target: &mut Target) {
    let mut random = Mt19937::new(parent.next_int_unbounded() as u32);
    if t.length <= 0 {
        t.length = 112 - (random.next_u32() % 28) as i32;
    }
    let room = t.step == -1;
    if room {
        t.step = t.length >> 1;
    }
    let mut branch = if t.length != 1 { (random.next_u32() % (t.length as u32 >> 1)) as i32 } else { 0 };
    let decay = if random.next_u32() % STEEP_CHANCE as u32 != 0 { 0.7 } else { 0.92 };
    if t.step >= t.length {
        return;
    }
    branch += t.length >> 2;
    let reach = (2.0 + t.thickness) + 16.0;
    let reach = reach * reach;
    let (cx, cz) = target.chunk;
    let centre = [(cx * 16 | 8) as f32, (cz * 16 | 8) as f32];
    let (mut pitch_change, mut yaw_change) = (0.0f32, 0.0f32);
    while t.step < t.length {
        let sine = mth::sin(t.step as f32 * PI / t.length as f32);
        let radius = sine * (0.0 + t.thickness) + 1.5;
        let (cos_pitch, sin_pitch) = (mth::cos(t.pitch), mth::sin(t.pitch));
        at[0] += mth::cos(t.yaw) * cos_pitch;
        at[1] += sin_pitch;
        at[2] += cos_pitch * mth::sin(t.yaw);
        t.pitch = pitch_change * 0.1 + t.pitch * decay;
        t.yaw = yaw_change * 0.1 + t.yaw;
        let (decayed_pitch_change, decayed_yaw_change) = (pitch_change * 0.9, yaw_change * 0.75);
        let d: [f32; 6] = std::array::from_fn(|_| random.next_float());
        let pitch_delta = d[2] * (d[0] - d[1]);
        pitch_change = pitch_delta + pitch_delta + decayed_pitch_change;
        yaw_change = d[5] * (d[3] - d[4]) * 4.0 + decayed_yaw_change;
        if !room {
            if t.thickness > 1.0 && t.step == branch {
                for side in [-FRAC_PI_2, FRAC_PI_2] {
                    let thickness = random.next_float() * 0.5 + 0.5;
                    let child = Tunnel { thickness, yaw: side + t.yaw, pitch: t.pitch / 3.0, step: branch, length: t.length, y_scale: 1.0 };
                    add_tunnel(&mut random, at, child, target);
                }
                return;
            }
            if random.next_u32() % SKIP_SEGMENT_CHANCE as u32 == 0 {
                t.step += 1;
                continue;
            }
        }
        let (dx, dz) = (at[0] - centre[0], at[2] - centre[1]);
        let left = (t.length - t.step) as f32;
        if dz * dz + dx * dx - left * left > reach {
            return;
        }
        let width = radius + radius;
        let outside = (centre[0] - 16.0) - width > at[0] || (centre[1] - 16.0) - width > at[2] || at[0] > (centre[0] + 16.0) + width || at[2] > width + (centre[1] + 16.0);
        // A room ends at its first ellipsoid that lava did not stop.
        if !outside && carve(target, at, radius, radius * t.y_scale) && room {
            return;
        }
        t.step += 1;
    }
}

/// BDS's floor: truncation, one less where that rounded up.
fn floor(value: f32) -> i32 {
    let truncated = value as i32;
    truncated - i32::from(truncated as f32 > value)
}

/// One ellipsoid; false when lava in or around its box stopped it (a room then walks on).
fn carve(target: &mut Target, [x, y, z]: [f32; 3], radius: f32, vertical: f32) -> bool {
    let (cx, cz) = (target.chunk.0 * 16, target.chunk.1 * 16);
    let x0 = (floor(x - radius) - cx - 1).max(0);
    let x1 = (floor(radius + x) - cx).min(15);
    let z0 = (floor(z - radius) - cz - 1).max(0);
    let z1 = (floor(radius + z) - cz).min(15);
    let y0 = floor(y - vertical).max(MIN_Y);
    let y1 = floor(vertical + y).min(MAX_Y);
    if x0 > x1 {
        return true;
    }
    if lava_near(target.shape, [x0, x1], [z0, z1], [y0, y1]) {
        return false;
    }
    for lx in x0..=x1 {
        let dx = ((cx + lx) as f32 + 0.5 - x) / radius;
        let dx = dx * dx;
        for lz in z0..=z1 {
            if y1 < y0 - 1 {
                continue;
            }
            let dz = ((cz + lz) as f32 + 0.5 - z) / radius;
            let dz = dz * dz;
            for cell in (y0 - 1..=y1).rev() {
                let dy = (cell as f32 + 0.5 - y) / vertical;
                if dy * dy + dx + dz < 1.0 && FLOOR_LEVEL < dy {
                    let (lx, lz) = (lx as usize, lz as usize);
                    if matches!(target.shape.block(lx, cell + 1, lz), Block::Netherrack | Block::Dirt | Block::CoarseDirt | Block::GrassBlock) {
                        target.shape.set_block(lx, cell + 1, lz, Block::Air);
                    }
                }
            }
        }
    }
    true
}

/// Lava within the box grown by two cells in y: whole columns on the box's rim, only the two end cells inside.
fn lava_near(shape: &ChunkShape, [x0, x1]: [i32; 2], [z0, z1]: [i32; 2], [y0, y1]: [i32; 2]) -> bool {
    for x in x0..=x1 {
        for z in z0..=z1 {
            let rim = x == x0 || x == x1 || z == z0 || z == z1;
            let mut y = y1 + 2;
            while y >= y0 - 2 {
                if (0..=127).contains(&y) && shape.block(x as usize, y, z as usize).is_lava() {
                    return true;
                }
                y = if rim || y == y0 - 2 { y - 1 } else { y0 - 2 };
            }
        }
    }
    false
}
