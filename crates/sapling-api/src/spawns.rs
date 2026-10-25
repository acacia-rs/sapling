//! A player's own spawn point, where it rises instead of at its world's spawn.

use crate::{BlockPos, PlayerId, ShardCtx};

impl ShardCtx<'_> {
    /// The block a player rises in when it dies, if it has one of its own in its world: the cell
    /// by the bed it last slept in, or one [`ShardCtx::set_spawn_point`] gave it.
    pub fn spawn_point(&self, player: PlayerId) -> Option<BlockPos> {
        self.backend.spawn_point(player)
    }

    /// Makes `at` in the world a player is in the block it rises in, telling its client, as
    /// `/spawnpoint` does; `None` takes its spawn point away, so it rises at the world's spawn.
    /// False for a player who is not in the world.
    pub fn set_spawn_point(&mut self, player: PlayerId, at: Option<BlockPos>) -> bool {
        self.backend.set_spawn_point(player, at)
    }
}

/// The spawn points side of [`crate::Backend`].
pub trait SpawnBackend {
    fn spawn_point(&self, player: PlayerId) -> Option<BlockPos>;
    fn set_spawn_point(&mut self, player: PlayerId, at: Option<BlockPos>) -> bool;
}

#[cfg(test)]
impl SpawnBackend for crate::testing::Fake {
    fn spawn_point(&self, _: PlayerId) -> Option<BlockPos> {
        None
    }
    fn set_spawn_point(&mut self, player: PlayerId, at: Option<BlockPos>) -> bool {
        self.log.push(format!("spawn point {} {at:?}", player.0));
        true
    }
}
