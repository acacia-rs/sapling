use bevy_ecs::prelude::*;

/// The shard's dice: xorshift, the same throws on every run.
#[derive(Resource)]
pub(crate) struct Dice(u64);

impl Default for Dice {
    fn default() -> Self {
        Self(0x9E37_79B9_7F4A_7C15)
    }
}

impl Dice {
    /// A number below `sides`.
    pub fn roll(&mut self, sides: u32) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 32) as u32) % sides.max(1)
    }

    /// A number from 0 up to 1.
    pub fn fraction(&mut self) -> f32 {
        self.roll(1 << 20) as f32 / (1 << 20) as f32
    }

    /// A number from the normal spread about 0 (Box-Muller).
    pub fn gaussian(&mut self) -> f32 {
        let (u, v) = (1.0 - self.fraction(), self.fraction());
        (-2.0 * u.ln()).sqrt() * (std::f32::consts::TAU * v).cos()
    }
}
