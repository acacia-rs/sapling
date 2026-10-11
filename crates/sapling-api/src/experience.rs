//! A player's experience: its level and how far it is into the next.

use crate::{PlayerId, ShardCtx};

impl ShardCtx<'_> {
    /// A player's level and how far into the next it is, 0 to 1.
    pub fn experience(&self, player: PlayerId) -> Option<(i32, f32)> {
        self.backend.experience(player)
    }

    /// Gives a player `points` of experience, as picking up an orb does. False for a player
    /// who is not in the world.
    pub fn give_experience(&mut self, player: PlayerId, points: u32) -> bool {
        self.backend.give_experience(player, points)
    }

    /// Adds `levels` to a player's level, or takes them if negative; a level cannot go under 0,
    /// and one brought there loses what it had of the next. False for a player not in the world.
    pub fn add_levels(&mut self, player: PlayerId, levels: i32) -> bool {
        self.backend.add_levels(player, levels)
    }
}

/// The experience side of [`crate::Backend`].
pub trait ExperienceBackend {
    fn experience(&self, player: PlayerId) -> Option<(i32, f32)>;
    fn give_experience(&mut self, player: PlayerId, points: u32) -> bool;
    fn add_levels(&mut self, player: PlayerId, levels: i32) -> bool;
}
