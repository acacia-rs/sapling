//! The ender dragon fight of an End world: read only, and told when a dragon dies or another is
//! summoned. The dragon itself is an entity like any other (`minecraft:ender_dragon`).

use crate::{EntityId, Event, ShardCtx};

/// Where an End's fight stands, as plain data.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DragonFight {
    /// The dragon, while one is alive: a dying one is not.
    pub dragon: Option<EntityId>,
    /// What it has left of its health.
    pub health: Option<f32>,
    /// End crystals standing on the spikes.
    pub crystals: u32,
    /// Whether a dragon of this End was ever killed.
    pub previously_killed: bool,
}

impl DragonFight {
    pub fn new(dragon: Option<EntityId>, health: Option<f32>, crystals: u32, previously_killed: bool) -> Self {
        Self { dragon, health, crystals, previously_killed }
    }
}

impl ShardCtx<'_> {
    /// The fight in the End of the world in scope; `None` where it has no End or no fight has
    /// begun there (nobody came near its main island yet).
    pub fn dragon_fight(&self) -> Option<DragonFight> {
        self.backend.dragon_fight()
    }
}

/// A dragon died: its death began, the exit portal opens when it ends. Offered in its End.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DragonKilledEvent {
    /// The dragon, an entity until its death ends.
    pub dragon: EntityId,
    /// No dragon of this End was killed before: the egg comes, and the most experience.
    pub first: bool,
}

impl DragonKilledEvent {
    pub fn new(dragon: EntityId, first: bool) -> Self {
        Self { dragon, first }
    }
}

impl Event for DragonKilledEvent {}

/// Four end crystals on the exit portal began summoning a dragon: the spikes are rebuilt, then
/// the dragon comes. Offered in its End.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DragonRespawnEvent {
    /// The crystals that began it.
    pub crystals: Vec<EntityId>,
}

impl DragonRespawnEvent {
    pub fn new(crystals: Vec<EntityId>) -> Self {
        Self { crystals }
    }
}

impl Event for DragonRespawnEvent {}
