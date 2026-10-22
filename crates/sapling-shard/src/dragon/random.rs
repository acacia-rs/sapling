//! The fight's own draws, so that a dragon flying every tick shifts no other roll of the shard
//! (`blocks::Dice`). BDS draws on the level's random: no run repeats another's choices.

/// Xorshift, seeded per End.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Random(u64);

impl Random {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A number below `bound`.
    pub fn next_int(&mut self, bound: u32) -> u32 {
        ((self.next() >> 32) as u32) % bound.max(1)
    }

    /// A number from 0 up to 1.
    pub fn next_float(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A normal draw, Marsaglia's polar way as BDS's (without its kept second value).
    pub fn next_gaussian(&mut self) -> f32 {
        loop {
            let (v1, v2) = (2.0 * self.next_float() - 1.0, 2.0 * self.next_float() - 1.0);
            let s = v1 * v1 + v2 * v2;
            if s < 1.0 && s != 0.0 {
                return v1 * (-2.0 * s.ln() / s).sqrt();
            }
        }
    }
}
