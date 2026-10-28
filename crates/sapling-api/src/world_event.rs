//! What the world does on its own that plugins may stop: explosions, mobs appearing, fire and
//! liquids spreading, plants growing. Each is offered with its world in focus, so a listener in
//! [`crate::Scope::World`] hears its own world's; none has a player.

use crate::{Block, BlockPos, Event};

/// Something is about to blow up. Cancelled, nothing is hurt or broken; the TNT or creeper that
/// made it is gone all the same.
#[derive(Debug, Clone, PartialEq)]
pub struct ExplosionEvent {
    at: [f32; 3],
    source: Option<String>,
    /// How hard it hits what is near.
    pub power: f32,
    /// The blocks it breaks, worked out from the power it was offered with; a listener may take
    /// some out or put others in.
    pub blocks: Vec<BlockPos>,
    cancelled: bool,
}

impl ExplosionEvent {
    pub fn new(at: [f32; 3], source: Option<String>, power: f32, blocks: Vec<BlockPos>) -> Self {
        Self { at, source, power, blocks, cancelled: false }
    }

    pub fn at(&self) -> [f32; 3] {
        self.at
    }

    /// The kind of entity that blew up (`minecraft:tnt`, `minecraft:creeper`, ..); `None` for a
    /// block, such as a respawn anchor.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

/// Why a mob is about to appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpawnReason {
    /// `ctx.spawn_entity`, `/summon`.
    Command,
    /// Another mob called it up: an evoker's vexes.
    Summoned,
    /// An effect on a body: oozing's slimes, infestation's silverfish.
    Effect,
    /// A slime or magma cube split as it died.
    Split,
    /// Natural spawning, by the world's spawn rules.
    Natural,
    /// A spawn egg used on a block.
    SpawnEgg,
    /// Where something thrown landed: an egg's chicks, an ender pearl's endermite.
    Thrown,
    /// Lightning turned another mob into it: a pig's zombified piglin, a villager's witch.
    Lightning,
    /// Built of blocks: a wither of soul sand and skulls.
    Built,
    /// A monster spawner or a trial spawner.
    Spawner,
    /// Born to two animals that bred.
    Bred,
    /// Another mob became it: a sheared mooshroom's cow.
    Transformed,
    /// A raid's wave.
    Raid,
}

/// A mob is about to appear. Cancelled, it does not. Mobs coming back with their column from
/// storage are not offered.
#[derive(Debug, Clone, PartialEq)]
pub struct EntitySpawnEvent {
    kind: String,
    feet: [f32; 3],
    reason: SpawnReason,
    cancelled: bool,
}

impl EntitySpawnEvent {
    pub fn new(kind: String, feet: [f32; 3], reason: SpawnReason) -> Self {
        Self { kind, feet, reason, cancelled: false }
    }

    /// Namespaced: `minecraft:zombie`.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn feet(&self) -> [f32; 3] {
        self.feet
    }

    pub fn reason(&self) -> SpawnReason {
        self.reason
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

/// Fire or a liquid is about to take the cell at [`BlockSpreadEvent::pos`] from
/// [`BlockSpreadEvent::source`]: fire spreading into it or burning its block away (the block is
/// air then), water or lava flowing into it. Cancelled, the cell keeps what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSpreadEvent {
    pos: BlockPos,
    source: BlockPos,
    block: Block,
    cancelled: bool,
}

impl BlockSpreadEvent {
    pub fn new(pos: BlockPos, source: BlockPos, block: Block) -> Self {
        Self { pos, source, block, cancelled: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    pub fn source(&self) -> BlockPos {
        self.source
    }

    /// What the cell would hold.
    pub fn block(&self) -> &Block {
        &self.block
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

/// A random tick is about to grow the plant at [`BlockGrowEvent::pos`]: a crop or stem ages or
/// bears fruit, cane, cactus or bamboo grows taller, a sapling becomes a tree, dirt turns to
/// grass. Cancelled, it stays as it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockGrowEvent {
    pos: BlockPos,
    plant: Block,
    cancelled: bool,
}

impl BlockGrowEvent {
    pub fn new(pos: BlockPos, plant: Block) -> Self {
        Self { pos, plant, cancelled: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    /// The block that grows, as it is now.
    pub fn plant(&self) -> &Block {
        &self.plant
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

macro_rules! stoppable {
    ($($event:ty),*) => {$(
        impl Event for $event {
            fn is_stopped(&self) -> bool {
                self.cancelled
            }
        }
    )*};
}

stoppable!(ExplosionEvent, EntitySpawnEvent, BlockSpreadEvent, BlockGrowEvent);
