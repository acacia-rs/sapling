//! rustc's FxHash for small integer keys: the aquifer's per-chunk maps hash grid cells millions
//! of times, where std's SipHash cost more than the lookups they guard.

use std::hash::{BuildHasherDefault, Hasher};

pub type FxBuild = BuildHasherDefault<FxHasher>;

#[derive(Default, Clone, Copy)]
pub struct FxHasher(u64);

impl FxHasher {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.add(u64::from(byte));
        }
    }

    fn write_i32(&mut self, value: i32) {
        self.add(u64::from(value as u32));
    }

    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
