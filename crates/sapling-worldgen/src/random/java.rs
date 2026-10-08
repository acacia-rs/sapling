//! `java.util.Random`, which BDS's v2 (data-driven) structures draw from (`JavaRandom` vtable
//! 14a886a70, nextInt 1459409e0).

use super::position_seed;

#[derive(Debug, Clone)]
pub struct JavaRandom(i64);

impl JavaRandom {
    const MULTIPLIER: i64 = 0x5_DEEC_E66D;
    const MASK: i64 = (1 << 48) - 1;

    pub fn new(seed: i64) -> Self {
        Self((seed ^ Self::MULTIPLIER) & Self::MASK)
    }

    /// Java's `WorldgenRandom.setLargeFeatureSeed`: a structure start's generator.
    pub fn large_feature(seed: i64, chunk_x: i32, chunk_z: i32) -> Self {
        let mut random = Self::new(seed);
        let a = random.next_long();
        let b = random.next_long();
        Self::new((chunk_x as i64).wrapping_mul(a) ^ (chunk_z as i64).wrapping_mul(b) ^ seed)
    }

    /// `forkPositional().at(x, y, z)` (145941160).
    pub fn fork_at(&mut self, x: i32, y: i32, z: i32) -> Self {
        let seed = self.next_long();
        Self::new(position_seed(x, y, z) ^ seed)
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.0 = (self.0.wrapping_mul(Self::MULTIPLIER).wrapping_add(0xB)) & Self::MASK;
        (self.0 >> (48 - bits)) as i32
    }

    pub fn next_int(&mut self, bound: i32) -> i32 {
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

    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1 << 24) as f32
    }

    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    /// Java's `Util.shuffle`: from the end, swap with `nextInt(i)`.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (2..=items.len()).rev() {
            let j = self.next_int(i as i32) as usize;
            items.swap(i - 1, j);
        }
    }
}
