//! The `minecraft:swamp` surface builder (build 1487e4d10, init 1487e4870): a puddle where a
//! 2-octave simplex noise is positive, then the overworld builder.

use super::column::Column;
use crate::block::Block;
use crate::noise::SimplexNoise;
use crate::random::Mt19937;

const SEA_LEVEL: i32 = 63;

pub struct PuddleNoise(Vec<SimplexNoise>);

impl PuddleNoise {
    /// An MT on the low 32 bits of the world seed.
    pub fn new(seed: i64) -> Self {
        let mut mt = Mt19937::new(seed as u32);
        Self(
            (0..2)
                .map(|_| {
                    let [a, b, c] = [mt.next_float(), mt.next_float(), mt.next_float()];
                    SimplexNoise::new([c, b, a], |bound| mt.next_int(bound as i32))
                })
                .collect(),
        )
    }

    fn value(&self, x: i32, z: i32) -> f32 {
        let (mut total, mut scale) = (0.0f32, 1.0f32);
        for octave in &self.0 {
            total = total + octave.value(x as f32 * 0.25 * scale, z as f32 * 0.25 * scale) / scale;
            scale *= 0.5;
        }
        total
    }
}

/// Floods the column's top block when the noise is positive and the top is no higher than 62 and
/// no deeper than `max_depth` below the sea.
pub fn build_puddle(column: &mut Column, noise: &PuddleNoise, max_depth: i32) {
    let (x, z) = column.world;
    if !(noise.value(x, z) > 0.0) || column.start > SEA_LEVEL {
        return;
    }
    let mut y = column.start - 1;
    while y >= SEA_LEVEL - max_depth {
        if column.get(y) != Block::Water {
            column.set(y, Block::Water);
            return;
        }
        y -= 1;
    }
}
