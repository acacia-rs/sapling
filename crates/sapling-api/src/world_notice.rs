//! The rest of what the world does with no player: liquid drying up, sounds, entities going and
//! worlds closing. Each is offered with its world in focus.

use crate::block_event::stoppable;
use crate::{Block, BlockPos, EntityId, Event};

/// A flowing liquid at `pos` is about to thin or dry up, its source gone. Cancelled, it stays
/// as it is until its next update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiquidDecayEvent {
    pos: BlockPos,
    before: Block,
    after: Block,
    cancelled: bool,
}

/// A sound is about to be heard by those near `at`: a block's, a mob's, a door's. Cancelled,
/// nobody hears it. Sounds a plugin plays ([`crate::ShardCtx::play_sound`]) are not offered.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundEvent {
    at: [f32; 3],
    name: String,
    cancelled: bool,
}

/// An entity left the world: removed, killed and gone, picked up, despawned, or unloaded with its
/// column. It is already gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDespawnEvent {
    id: EntityId,
    kind: String,
}

/// A world is closing: removed, or its shard stopping. Its players are still in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldCloseEvent {
    name: String,
}

impl Event for EntityDespawnEvent {}
impl Event for WorldCloseEvent {}

stoppable!(LiquidDecayEvent, SoundEvent);

impl LiquidDecayEvent {
    pub fn new(pos: BlockPos, before: Block, after: Block) -> Self {
        Self { pos, before, after, cancelled: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    pub fn before(&self) -> &Block {
        &self.before
    }

    /// Thinner liquid, or air where it dried up.
    pub fn after(&self) -> &Block {
        &self.after
    }
}

impl SoundEvent {
    pub fn new(at: [f32; 3], name: String) -> Self {
        Self { at, name, cancelled: false }
    }

    pub fn at(&self) -> [f32; 3] {
        self.at
    }

    /// LevelSoundEvent's name: `break`, `place`, `hit`, `door.open`, ..
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl EntityDespawnEvent {
    pub fn new(id: EntityId, kind: String) -> Self {
        Self { id, kind }
    }

    pub fn id(&self) -> EntityId {
        self.id
    }

    /// Namespaced, as [`crate::EntityInfo::kind`].
    pub fn kind(&self) -> &str {
        &self.kind
    }
}

impl WorldCloseEvent {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
