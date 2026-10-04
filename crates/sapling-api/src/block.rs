//! Blocks as plugins see them: a position, a face, and a state by name.

use crate::event::Cancellable;

/// A block's place in the world: x, y, z.
pub type BlockPos = [i32; 3];

/// A side of a block, numbered as the client numbers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Down,
    Up,
    /// Towards -z.
    North,
    /// Towards +z.
    South,
    /// Towards -x.
    West,
    /// Towards +x.
    East,
}

impl Face {
    pub const ALL: [Face; 6] = [Face::Down, Face::Up, Face::North, Face::South, Face::West, Face::East];

    pub fn from_index(index: u32) -> Option<Self> {
        Self::ALL.get(index as usize).copied()
    }

    pub fn offset(self) -> [i32; 3] {
        match self {
            Face::Down => [0, -1, 0],
            Face::Up => [0, 1, 0],
            Face::North => [0, 0, -1],
            Face::South => [0, 0, 1],
            Face::West => [-1, 0, 0],
            Face::East => [1, 0, 0],
        }
    }

    /// The position this face of the block at `pos` touches: where a block placed against it goes.
    pub fn neighbour(self, pos: BlockPos) -> BlockPos {
        let offset = self.offset();
        [pos[0].saturating_add(offset[0]), pos[1].saturating_add(offset[1]), pos[2].saturating_add(offset[2])]
    }
}

/// A block state: a name like `minecraft:oak_stairs` and its properties as `key=value` pairs
/// sorted by key and separated by commas (`upside_down_bit=0,weirdo_direction=2`), booleans as
/// `0`/`1`. Empty properties name the block's first state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Block {
    pub name: String,
    pub properties: String,
}

impl Block {
    pub const AIR: &'static str = "minecraft:air";

    /// The first state of a block; a name without a namespace is a `minecraft:` one.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let name = if name.contains(':') { name } else { format!("minecraft:{name}") };
        Self { name, properties: String::new() }
    }

    pub fn with(mut self, properties: impl Into<String>) -> Self {
        self.properties = properties.into();
        self
    }

    pub fn air() -> Self {
        Self::new(Self::AIR)
    }

    pub fn is_air(&self) -> bool {
        self.name == Self::AIR
    }
}

/// A player is about to break a block. Allowed, the block becomes air (or the water it stood
/// in) for everyone; cancelled, the breaker's client is shown the block again.
///
/// Starts cancelled for players in adventure and spectator mode: [`BreakEvent::allow`] lets one
/// through. See [`crate::Priority`] for which handlers still run after a cancel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakEvent {
    pos: BlockPos,
    block: Block,
    allowed: bool,
    stopped: bool,
}

/// A player is about to place a block. Allowed, [`PlaceEvent::block`] appears at
/// [`PlaceEvent::pos`] for everyone; cancelled, the placer's client is shown what is really
/// there and at the block it clicked.
///
/// The block is the one the client says it holds (there are no inventories yet), so the event
/// starts cancelled for everyone but players in creative mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceEvent {
    pos: BlockPos,
    against: BlockPos,
    face: Face,
    /// What will be placed; a handler may put another block here.
    pub block: Block,
    allowed: bool,
    stopped: bool,
}

/// A player clicked a block without placing one: a door, a lever, a chest. Nothing happens to
/// such blocks yet, allowed or not; the client is shown the block as it is, so a door it opened
/// on its own screen closes again. Starts cancelled for spectators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractEvent {
    pos: BlockPos,
    face: Face,
    block: Block,
    allowed: bool,
    stopped: bool,
}

impl BreakEvent {
    pub fn new(pos: BlockPos, block: Block, allowed: bool) -> Self {
        Self { pos, block, allowed, stopped: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    /// The block being broken.
    pub fn block(&self) -> &Block {
        &self.block
    }
}

impl PlaceEvent {
    pub fn new(pos: BlockPos, against: BlockPos, face: Face, block: Block, allowed: bool) -> Self {
        Self { pos, against, face, block, allowed, stopped: false }
    }

    /// Where the block goes.
    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    /// The block that was clicked, and which face of it.
    pub fn against(&self) -> (BlockPos, Face) {
        (self.against, self.face)
    }
}

impl InteractEvent {
    pub fn new(pos: BlockPos, face: Face, block: Block, allowed: bool) -> Self {
        Self { pos, face, block, allowed, stopped: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    pub fn face(&self) -> Face {
        self.face
    }

    /// The block that was clicked.
    pub fn block(&self) -> &Block {
        &self.block
    }
}

macro_rules! block_event {
    ($($event:ty),*) => {$(
        impl $event {
            /// Refuses the event; only `Monitor` handlers still see it.
            pub fn cancel(&mut self) {
                (self.allowed, self.stopped) = (false, true);
            }

            /// Lets the event happen, also one that started cancelled. Later handlers still run
            /// and may cancel it.
            pub fn allow(&mut self) {
                (self.allowed, self.stopped) = (true, false);
            }

            pub fn is_cancelled(&self) -> bool {
                !self.allowed
            }
        }

        impl Cancellable for $event {
            fn is_cancelled(&self) -> bool {
                !self.allowed
            }

            fn is_stopped(&self) -> bool {
                self.stopped
            }
        }
    )*};
}
block_event!(BreakEvent, PlaceEvent, InteractEvent);
