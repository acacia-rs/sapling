use crate::{PlayerId, ShardCtx};

/// What hurt a player: it picks the flash its client shows and the words of its death.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageCause {
    /// `/damage` without a cause, `/kill`.
    Generic,
    Attack,
    Suffocation,
    Fall,
    /// Touching fire.
    Fire,
    /// Burning on.
    FireTick,
    Lava,
    Drowning,
}

/// Players in survival and adventure have 20 health; the shard takes from it for falls, fire and
/// lava, and a player with none left is dead until its client asks to rise at the world's spawn.
impl ShardCtx<'_> {
    /// A player's health, 0 to 20; `None` for one who is not in the world.
    pub fn health(&self, player: PlayerId) -> Option<f32> {
        self.backend.health(player)
    }

    /// Takes `amount` from a player's health as a hit of `cause`. False where it took nothing:
    /// the player is dead, in creative or spectator, or was hit as hard less than half a second ago.
    pub fn hurt_player(&mut self, player: PlayerId, amount: f32, cause: DamageCause) -> bool {
        self.backend.hurt_player(player, amount, cause)
    }

    /// Kills a player as `/kill` does. False for one already dead, or in creative or spectator.
    pub fn kill_player(&mut self, player: PlayerId) -> bool {
        self.backend.kill_player(player)
    }
}

/// The health side of [`crate::Backend`].
pub trait HealthBackend {
    fn health(&self, player: PlayerId) -> Option<f32>;
    fn hurt_player(&mut self, player: PlayerId, amount: f32, cause: DamageCause) -> bool;
    fn kill_player(&mut self, player: PlayerId) -> bool;
}
