//! One Perlin octave (Java `ImprovedNoise`, BDS `ParityImprovedNoiseImpl`), in f32.

use super::NoiseRandom;

const GRADIENT: [[f32; 3]; 16] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
    [1.0, 1.0, 0.0],
    [0.0, -1.0, 1.0],
    [-1.0, 1.0, 0.0],
    [0.0, -1.0, -1.0],
];

#[derive(Debug, Clone)]
pub struct ImprovedNoise {
    pub xo: f32,
    pub yo: f32,
    pub zo: f32,
    p: [u8; 256],
}

impl ImprovedNoise {
    /// Offsets from `nextDouble() * 256` in f64, then rounded (145e24e60).
    pub fn new(random: &mut impl NoiseRandom) -> Self {
        let xo = (random.next_double() * 256.0) as f32;
        let yo = (random.next_double() * 256.0) as f32;
        let zo = (random.next_double() * 256.0) as f32;
        Self::with_offsets(random, xo, yo, zo)
    }

    /// BlendedNoise's octaves take `nextFloat() * 256` instead (145e29ee0).
    pub fn new_blended(random: &mut impl NoiseRandom) -> Self {
        let xo = random.next_float() * 256.0;
        let yo = random.next_float() * 256.0;
        let zo = random.next_float() * 256.0;
        Self::with_offsets(random, xo, yo, zo)
    }

    fn with_offsets(random: &mut impl NoiseRandom, xo: f32, yo: f32, zo: f32) -> Self {
        let mut p: [u8; 256] = std::array::from_fn(|i| i as u8);
        for i in 0..256 {
            let j = random.next_int_bounded(256 - i as u32) as usize;
            p.swap(i, i + j);
        }
        Self { xo, yo, zo, p }
    }

    fn hash(&self, i: i32) -> i32 {
        self.p[(i & 0xFF) as usize] as i32
    }

    /// Noise at an offset-applied position split into its floor and fraction.
    pub fn sample_split(&self, floor: [i32; 3], frac: [f32; 3]) -> f32 {
        let [x, y, z] = floor;
        let [dx, dy, dz] = frac;
        let a = self.hash(x);
        let b = self.hash(x + 1);
        let aa = self.hash(a + y);
        let ab = self.hash(a + y + 1);
        let ba = self.hash(b + y);
        let bb = self.hash(b + y + 1);

        let d000 = grad_dot(self.hash(aa + z), dx, dy, dz);
        let d100 = grad_dot(self.hash(ba + z), dx - 1.0, dy, dz);
        let d010 = grad_dot(self.hash(ab + z), dx, dy - 1.0, dz);
        let d110 = grad_dot(self.hash(bb + z), dx - 1.0, dy - 1.0, dz);
        let d001 = grad_dot(self.hash(aa + z + 1), dx, dy, dz - 1.0);
        let d101 = grad_dot(self.hash(ba + z + 1), dx - 1.0, dy, dz - 1.0);
        let d011 = grad_dot(self.hash(ab + z + 1), dx, dy - 1.0, dz - 1.0);
        let d111 = grad_dot(self.hash(bb + z + 1), dx - 1.0, dy - 1.0, dz - 1.0);

        let (sx, sy, sz) = (smoothstep(dx), smoothstep(dy), smoothstep(dz));
        lerp3(sx, sy, sz, [d000, d100, d010, d110, d001, d101, d011, d111])
    }

    /// Noise at a position the caller already shifted by `xo/yo/zo`.
    pub fn sample_offset(&self, x: f32, y: f32, z: f32) -> f32 {
        let (ix, iy, iz) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
        self.sample_split([ix, iy, iz], [x - ix as f32, y - iy as f32, z - iz as f32])
    }

    pub fn hash_at(&self, i: i32) -> i32 {
        self.hash(i)
    }
}

pub(super) fn grad_dot(hash: i32, x: f32, y: f32, z: f32) -> f32 {
    let g = GRADIENT[(hash & 15) as usize];
    g[0] * x + g[1] * y + g[2] * z
}

/// BDS order: the quintic factor times the cube (145e26950, 145e23fc0).
pub fn smoothstep(t: f32) -> f32 {
    let cube = t * t * t;
    ((t * 6.0 - 15.0) * t + 10.0) * cube
}

fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

fn lerp3(tx: f32, ty: f32, tz: f32, d: [f32; 8]) -> f32 {
    let x00 = lerp(tx, d[0], d[1]);
    let x10 = lerp(tx, d[2], d[3]);
    let x01 = lerp(tx, d[4], d[5]);
    let x11 = lerp(tx, d[6], d[7]);
    lerp(tz, lerp(ty, x00, x10), lerp(ty, x01, x11))
}
