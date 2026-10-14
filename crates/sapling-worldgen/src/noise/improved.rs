//! One Perlin octave (Java `ImprovedNoise`, BDS `ParityImprovedNoiseImpl`), in f32.

use super::NoiseRandom;

pub(super) const GRADIENT: [[f32; 3]; 16] = [
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
    /// `p[i] | p[i + 1] << 8` (i + 1 wrapping), for the AVX2 lanes' gathers (avx2.rs).
    pub(super) pairs: [i32; 256],
}

impl ImprovedNoise {
    /// Offsets from `nextDouble() * 256` in f64, then rounded (145e24e60).
    pub fn new(random: &mut impl NoiseRandom) -> Self {
        let xo = (random.next_double() * 256.0) as f32;
        let yo = (random.next_double() * 256.0) as f32;
        let zo = (random.next_double() * 256.0) as f32;
        Self::with_offsets(random, xo, yo, zo)
    }

    /// BlendedNoise's octaves take `nextFloat() * 256` instead (145e29ee0), and the Xoroshiro
    /// generator's virtual `nextFloat` is its 53-bit `nextDouble` rounded to f32, not Java's 24 bits:
    /// the octave objects of a live BDS hold these offsets to the bit (examples/octave_offsets.rs).
    /// The 24-bit float lost up to 2^-16 per offset, which the low octaves multiply by up to 32768.
    pub fn new_blended(random: &mut impl NoiseRandom) -> Self {
        let xo = random.next_double() as f32 * 256.0;
        let yo = random.next_double() as f32 * 256.0;
        let zo = random.next_double() as f32 * 256.0;
        Self::with_offsets(random, xo, yo, zo)
    }

    fn with_offsets(random: &mut impl NoiseRandom, xo: f32, yo: f32, zo: f32) -> Self {
        let mut p: [u8; 256] = std::array::from_fn(|i| i as u8);
        for i in 0..256 {
            let j = random.next_int_bounded(256 - i as u32) as usize;
            p.swap(i, i + j);
        }
        let pairs = std::array::from_fn(|i| i32::from(p[i]) | i32::from(p[(i + 1) & 0xFF]) << 8);
        Self { xo, yo, zo, p, pairs }
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
        let (ix, iy, iz) = (super::floor(x) as i32, super::floor(y) as i32, super::floor(z) as i32);
        self.sample_split([ix, iy, iz], [x - ix as f32, y - iy as f32, z - iz as f32])
    }

    pub fn hash_at(&self, i: i32) -> i32 {
        self.hash(i)
    }

    /// `sample_offset` at `LANES` positions: the table lookups lane by lane, the arithmetic in phases
    /// over the lanes with the scalar path's operations, so each lane is bit-identical to it.
    pub fn sample_offset_lanes(&self, x: &Lanes, y: &Lanes, z: &Lanes) -> Lanes {
        #[cfg(target_arch = "x86_64")]
        if let Some(lanes) = super::avx2::sample_offset(self, x, y, z) {
            return lanes;
        }
        self.sample_offset_each(x, y, z)
    }

    pub(super) fn sample_offset_each(&self, x: &Lanes, y: &Lanes, z: &Lanes) -> Lanes {
        let floor = |v: &Lanes| -> [i32; LANES] { std::array::from_fn(|l| super::floor(v[l]) as i32) };
        let (ix, iy, iz) = (floor(x), floor(y), floor(z));
        let dx: Lanes = std::array::from_fn(|l| x[l] - ix[l] as f32);
        let dy: Lanes = std::array::from_fn(|l| y[l] - iy[l] as f32);
        let dz: Lanes = std::array::from_fn(|l| z[l] - iz[l] as f32);
        // Per lane, the gradients of the corners 000, 100, 010, 110, 001, 101, 011, 111.
        let mut gradients = [[[0.0f32; LANES]; 3]; 8];
        for l in 0..LANES {
            let (a, b) = (self.hash(ix[l]), self.hash(ix[l] + 1));
            let (aa, ab, ba, bb) = (self.hash(a + iy[l]), self.hash(a + iy[l] + 1), self.hash(b + iy[l]), self.hash(b + iy[l] + 1));
            let corners = [aa, ba, ab, bb].map(|h| h + iz[l]);
            for (k, &h) in corners.iter().chain(&corners.map(|h| h + 1)).enumerate() {
                let g = GRADIENT[(self.hash(h) & 15) as usize];
                (0..3).for_each(|axis| gradients[k][axis][l] = g[axis]);
            }
        }
        let dot = |k: usize, ox: f32, oy: f32, oz: f32| -> Lanes {
            let g = &gradients[k];
            std::array::from_fn(|l| g[0][l] * (dx[l] - ox) + g[1][l] * (dy[l] - oy) + g[2][l] * (dz[l] - oz))
        };
        let d = [dot(0, 0.0, 0.0, 0.0), dot(1, 1.0, 0.0, 0.0), dot(2, 0.0, 1.0, 0.0), dot(3, 1.0, 1.0, 0.0), dot(4, 0.0, 0.0, 1.0), dot(5, 1.0, 0.0, 1.0), dot(6, 0.0, 1.0, 1.0), dot(7, 1.0, 1.0, 1.0)];
        std::array::from_fn(|l| lerp3(smoothstep(dx[l]), smoothstep(dy[l]), smoothstep(dz[l]), d.map(|d| d[l])))
    }
}

/// Positions a lane sampler takes at once.
pub const LANES: usize = 8;
pub type Lanes = [f32; LANES];

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
