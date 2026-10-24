//! How far the server takes a player's word for where it is: per player, over the shard's default.

use crate::{PlayerId, ShardCtx};

/// What the server makes of where a client says its player is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MovementPolicy {
    /// Believed, past sanity checks: for a minigame with its own rules of motion.
    Trust,
    /// Also stepped by the server with the game's physics, and logged where the two part; the
    /// client's word still stands.
    #[default]
    Observe,
    /// Stepped by the server, and corrected where the client strays past the shard's threshold, as
    /// BDS's server-authoritative movement does.
    Correct,
}

impl ShardCtx<'_> {
    /// The policy a player's movement is under: its own, or the shard's.
    pub fn movement_policy(&self, player: PlayerId) -> Option<MovementPolicy> {
        self.backend.movement_policy(player)
    }

    /// Puts a player's movement under `policy`, or back under the shard's with `None`; false for a
    /// player not on this shard. The server's step starts afresh from where the client is.
    pub fn set_movement_policy(&mut self, player: PlayerId, policy: Option<MovementPolicy>) -> bool {
        self.backend.set_movement_policy(player, policy)
    }
}

/// The movement side of [`crate::Backend`].
pub trait MovementBackend {
    fn movement_policy(&self, player: PlayerId) -> Option<MovementPolicy>;
    fn set_movement_policy(&mut self, player: PlayerId, policy: Option<MovementPolicy>) -> bool;
}

#[cfg(test)]
impl MovementBackend for crate::testing::Fake {
    fn movement_policy(&self, _: PlayerId) -> Option<MovementPolicy> {
        None
    }
    fn set_movement_policy(&mut self, player: PlayerId, policy: Option<MovementPolicy>) -> bool {
        self.log.push(format!("movement {} {policy:?}", player.0));
        true
    }
}
