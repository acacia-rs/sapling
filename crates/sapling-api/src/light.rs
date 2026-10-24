//! Light as the server reckons it for its rules: what grows, spawns and melts. The client works
//! out its own for what it draws.

use crate::{BlockPos, ShardCtx};

/// The two kinds of light at a cell, 0 to 15 each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Light {
    /// From the sky, at full strength whatever the time of day.
    pub sky: u8,
    /// From torches, lava, glowstone and the rest that shines.
    pub block: u8,
}

impl ShardCtx<'_> {
    /// The light at a cell of the world in scope, as worked out at the start of the tick and again
    /// before random ticks: a block written since shows from the next of those. `None` where
    /// nothing is loaded.
    pub fn light_at(&self, pos: BlockPos) -> Option<Light> {
        self.backend.light_at(pos)
    }

    /// What the game's rules read as a cell's light: its sky light dimmed by the night and the
    /// weather, or its block light where that is brighter; 0 inside a block that stops all light.
    /// A sapling grows from 9 over it.
    pub fn brightness_at(&self, pos: BlockPos) -> Option<u8> {
        self.backend.brightness_at(pos)
    }
}

/// The light side of [`crate::Backend`].
pub trait LightBackend {
    fn light_at(&self, pos: BlockPos) -> Option<Light>;
    fn brightness_at(&self, pos: BlockPos) -> Option<u8>;
}

#[cfg(test)]
impl LightBackend for crate::testing::Fake {
    fn light_at(&self, _: BlockPos) -> Option<Light> {
        None
    }
    fn brightness_at(&self, _: BlockPos) -> Option<u8> {
        None
    }
}
