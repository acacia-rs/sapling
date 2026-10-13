//! Noise checks against Java values (deepslate's tests, taken from Java 1.18) within f32 tolerance.

use super::*;

/// `java.util.Random`, only to reproduce Java's legacy-seeded test noises.
struct JavaRandom(i64);

impl JavaRandom {
    fn new(seed: i64) -> Self {
        Self((seed ^ 0x5DEECE66D) & ((1 << 48) - 1))
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.0 = (self.0.wrapping_mul(0x5DEECE66D).wrapping_add(0xB)) & ((1 << 48) - 1);
        (self.0 >> (48 - bits)) as i32
    }
}

impl NoiseRandom for JavaRandom {
    fn next_double(&mut self) -> f64 {
        (((self.next(26) as i64) << 27) + self.next(27) as i64) as f64 * 1.1102230246251565e-16
    }

    fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1 << 24) as f32
    }

    fn next_int_bounded(&mut self, bound: u32) -> i32 {
        let bound = bound as i32;
        if bound & -bound == bound {
            return ((bound as i64 * self.next(31) as i64) >> 31) as i32;
        }
        loop {
            let bits = self.next(31);
            let value = bits % bound;
            if bits - value + (bound - 1) >= 0 {
                return value;
            }
        }
    }
}

#[test]
fn improved_noise_matches_java() {
    let noise = ImprovedNoise::new(&mut JavaRandom::new(845));
    assert!((noise.xo - 179.49112).abs() < 1e-4);
    assert!((noise.yo - 178.89801).abs() < 1e-4);
    assert!((noise.zo - 139.89345).abs() < 1e-4);
    let at = |x: f32, y: f32, z: f32| noise.sample_offset(x + noise.xo, y + noise.yo, z + noise.zo);
    assert!((at(0.0, 0.0, 0.0) - 0.009862268).abs() < 1e-4);
    assert!((at(0.5, 4.0, -2.0) - -0.11885865).abs() < 1e-4);
    assert!((at(-204.0, 28.0, 12.0) - -0.58968128).abs() < 1e-3);
}

#[test]
fn lane_samples_are_bit_identical() {
    let mut random = JavaRandom::new(20261003);
    let noise = NormalNoise::from_mt(&mut crate::random::Mt19937::new(7), -7, &[1.0, 0.0, 2.0]);
    const SPECIAL: [f32; 8] = [0.0, -0.0, -0.5, 8_388_607.5, -8_388_609.0, 1.0e9, -1.9e9, 3.0e9];
    for round in 0..400 {
        let mut next = || if round % 4 == 0 { SPECIAL[random.next(3) as usize] } else { (random.next_float() - 0.5) * 2.0e5 };
        let (x, y, z): (Lanes, Lanes, Lanes) = (std::array::from_fn(|_| next()), std::array::from_fn(|_| next()), std::array::from_fn(|_| next()));
        let lanes = noise.sample_lanes(&x, &y, &z);
        for l in 0..LANES {
            assert_eq!(lanes[l].to_bits(), noise.sample(x[l], y[l], z[l]).to_bits());
        }
    }
}
