//! Java's `SimplexNoise` as BDS computes it in f32 (2D 145e27a70, 3D 145e27d50, built by 145e2a730).

/// Set at startup by 145e2d940 (0x3ebb67ae): one ulp under the f32 nearest (sqrt(3) - 1) / 2.
const F2: f32 = f32::from_bits(0x3ebb_67ae);
const G2: f32 = 0.211_324_87;

const GRADIENTS: [[f32; 3]; 12] = [
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
];

/// `count` octaves as `PerlinSimplexNoise`'s legacy constructor (145e2a730 with its flag clear) draws them off an
/// MT: three `nextFloat`s taken z, y, x, then the permutation.
pub fn legacy_simplex_octaves(random: &mut crate::random::Mt19937, count: usize) -> Vec<SimplexNoise> {
    (0..count)
        .map(|_| {
            let [a, b, c] = [random.next_float(), random.next_float(), random.next_float()];
            SimplexNoise::new([c, b, a], |bound| random.next_int(bound as i32))
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct SimplexNoise {
    /// Added to the coordinates by the caller.
    pub origin: [f32; 2],
    permutation: [u8; 256],
}

/// BDS's floor: truncation, minus one for every value not above zero (so 0.0 floors to -1).
fn floor(value: f32) -> i32 {
    value as i32 - (value <= 0.0) as i32
}

impl SimplexNoise {
    /// `offsets` are three `nextFloat()` draws already in x, y, z order; `next_int(bound)` then
    /// shuffles the permutation as Java does.
    pub fn new(offsets: [f32; 3], mut next_int: impl FnMut(u32) -> i32) -> Self {
        let mut permutation: [u8; 256] = std::array::from_fn(|i| i as u8);
        for i in 0..256 {
            let j = i + next_int(256 - i as u32) as usize;
            permutation.swap(i, j);
        }
        Self { origin: [offsets[0] * 256.0, offsets[1] * 256.0], permutation }
    }

    fn p(&self, i: i32) -> i32 {
        self.permutation[(i & 255) as usize] as i32
    }

    fn corner(&self, gradient: i32, x: f32, y: f32) -> f32 {
        let t = 0.5 - x * x - y * y;
        if 0.0 > t {
            return 0.0;
        }
        let [gx, gy, _] = GRADIENTS[(gradient % 12) as usize];
        (y * gy + x * gx) * (t * t * t * t)
    }

    pub fn value(&self, x: f32, y: f32) -> f32 {
        let s = (x + y) * F2;
        let (i, j) = (floor(x + s), floor(s + y));
        let t = (i + j) as f32 * G2;
        let (x0, y0) = (x - (i as f32 - t), y - (j as f32 - t));
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let (x1, y1) = (x0 - i1 as f32 + G2, y0 - j1 as f32 + G2);
        let g2 = G2 + G2;
        let (x2, y2) = (x0 + -1.0 + g2, y0 + -1.0 + g2);
        let (ii, jj) = (i & 255, j & 255);
        let n0 = self.corner(self.p(ii + self.p(jj)), x0, y0);
        let n1 = self.corner(self.p(ii + i1 + self.p(jj + j1)), x1, y1);
        let n2 = self.corner(self.p(ii + 1 + self.p(jj + 1)), x2, y2);
        (n0 + n1 + n2) * 70.0
    }

    fn corner3(&self, gradient: i32, [x, y, z]: [f32; 3]) -> f32 {
        let t = 0.6 - x * x - y * y - z * z;
        if 0.0 > t {
            return 0.0;
        }
        let [gx, gy, gz] = GRADIENTS[(gradient % 12) as usize];
        (y * gy + x * gx + z * gz) * (t * t * t * t)
    }

    pub fn value3(&self, x: f32, y: f32, z: f32) -> f32 {
        const G3: f32 = 0.166_666_67;
        let s = (x + y + z) * 0.333_333_34;
        let (i, j, k) = (floor(x + s), floor(y + s), floor(s + z));
        let t = (i + j + k) as f32 * G3;
        let (x0, y0, z0) = (x - (i as f32 - t), y - (j as f32 - t), z - (k as f32 - t));
        let ([i1, j1, k1], [i2, j2, k2]) = if x0 >= y0 {
            if y0 >= z0 { ([1, 0, 0], [1, 1, 0]) } else if x0 >= z0 { ([1, 0, 0], [1, 0, 1]) } else { ([0, 0, 1], [1, 0, 1]) }
        } else if z0 > y0 {
            ([0, 0, 1], [0, 1, 1])
        } else if z0 > x0 {
            ([0, 1, 0], [0, 1, 1])
        } else {
            ([0, 1, 0], [1, 1, 0])
        };
        let offset = |a: f32, b: i32, c: f32| a - b as f32 + c;
        let c1 = [offset(x0, i1, G3), offset(y0, j1, G3), offset(z0, k1, G3)];
        let c2 = [offset(x0, i2, 0.333_333_34), offset(y0, j2, 0.333_333_34), offset(z0, k2, 0.333_333_34)];
        let c3 = [x0 + -1.0 + 0.5, y0 + -1.0 + 0.5, z0 + -1.0 + 0.5];
        let (ii, jj, kk) = (i & 255, j & 255, k & 255);
        let hash = |a: i32, b: i32, c: i32| self.p(ii + a + self.p(jj + b + self.p(kk + c)));
        let n0 = self.corner3(hash(0, 0, 0), [x0, y0, z0]);
        let n1 = self.corner3(hash(i1, j1, k1), c1);
        let n2 = self.corner3(hash(i2, j2, k2), c2);
        let n3 = self.corner3(hash(1, 1, 1), c3);
        (n0 + n1 + n2 + n3) * 32.0
    }
}
