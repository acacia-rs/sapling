//! The ten obsidian spikes around the main island (`getSpikesForLevel` 148273530): a shuffle of their sizes on an MT
//! seeded with the level seed, centres on a radius-42 circle through the Mth table, truncated.

use crate::mth;
use crate::random::Mt19937;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spike {
    pub x: i32,
    pub z: i32,
    pub radius: i32,
    /// The obsidian's height: it fills y 0 up to `height - 1`.
    pub height: i32,
    /// An iron bar cage stands on top.
    pub guarded: bool,
}

/// The spikes of `level_seed` (its low 32 bits) in BDS's list order.
pub fn spikes(level_seed: u32) -> [Spike; 10] {
    let mut random = Mt19937::new(level_seed);
    let mut sizes: [i32; 10] = std::array::from_fn(|i| i as i32);
    for i in 1..10 {
        let j = (random.next_u32() % (i as u32 + 1)) as usize;
        sizes.swap(i, j);
    }
    std::array::from_fn(|i| {
        let angle = i as f32 * 0.314_159_27 + -std::f32::consts::PI;
        let turn = angle + angle;
        let size = sizes[i];
        Spike { x: (mth::cos(turn) * 42.0) as i32, z: (mth::sin(turn) * 42.0) as i32, radius: size / 3 + 2, height: size * 3 + 76, guarded: size == 1 || size == 2 }
    })
}
