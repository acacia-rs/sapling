//! `Density::preliminary_surface` per quart column in a lock-free direct-mapped table: the surface,
//! the aquifer, the structure stage and decoration ask the same columns hundreds of times per chunk,
//! and each answer costs some forty noise octaves. A slot lost to a collision is only recomputed.

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

const BITS: u32 = 14;
/// Quart coordinates are packed in 24 bits each.
const LIMIT: i32 = 1 << 23;
/// Values are stored biased so that a zero slot is empty.
const BIAS: i32 = 0x100;

pub struct SurfaceCache(Box<[AtomicU64]>);

impl SurfaceCache {
    pub fn new() -> Self {
        Self((0..1usize << BITS).map(|_| AtomicU64::new(0)).collect())
    }

    /// `compute`'s value, which must lie in `-BIAS + 1..0xFFFF - BIAS`.
    pub fn get(&self, qx: i32, qz: i32, compute: impl FnOnce() -> i32) -> i32 {
        if !(-LIMIT..LIMIT).contains(&qx) || !(-LIMIT..LIMIT).contains(&qz) {
            return compute();
        }
        let key = (qx as u64 & 0xFF_FFFF) << 40 | (qz as u64 & 0xFF_FFFF) << 16;
        let mix = (qx as u32).wrapping_mul(0x9E37_79B1).wrapping_add(qz as u32).wrapping_mul(0x85EB_CA6B);
        let slot = &self.0[(mix >> (32 - BITS)) as usize];
        let entry = slot.load(Relaxed);
        if entry & !0xFFFF == key && entry & 0xFFFF != 0 {
            return (entry & 0xFFFF) as i32 - BIAS;
        }
        let value = compute();
        slot.store(key | (value + BIAS) as u64, Relaxed);
        value
    }
}
