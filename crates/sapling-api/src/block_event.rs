//! More of what the world does on its own (world_event.rs): lava hardening, farmland trodden,
//! leaves decaying, portals lighting, redstone changing. Each is offered with its world in focus
//! and has no player. Fire spreading and burning, and liquids flowing, are
//! [`crate::BlockSpreadEvent`]s.

use crate::{Block, BlockPos, Event};

/// Lava at [`LiquidHardenEvent::pos`] is about to harden where water meets it: to obsidian,
/// cobblestone, stone or basalt. Cancelled, it stays lava.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiquidHardenEvent {
    pos: BlockPos,
    into: Block,
    cancelled: bool,
}

/// Farmland is about to be trodden to dirt by something that fell onto it. Cancelled, it stays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CropTrampleEvent {
    pos: BlockPos,
    cancelled: bool,
}

/// Leaves no wood holds any more are about to decay at their random tick. Cancelled, they stay
/// (and are asked about again at a later tick).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeavesDecayEvent {
    pos: BlockPos,
    leaves: Block,
    cancelled: bool,
}

/// Fire is about to light a nether portal in a frame. Cancelled, no portal forms; the fire is
/// lit as in any other place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortalCreateEvent {
    cells: Vec<BlockPos>,
    cancelled: bool,
}

/// Redstone power changed what the block at [`RedstoneUpdateEvent::pos`] shows: dust's signal,
/// a lamp, a door, a repeater. Told after circuits were worked out; it cannot be stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedstoneUpdateEvent {
    pos: BlockPos,
}

impl Event for RedstoneUpdateEvent {}

impl LiquidHardenEvent {
    pub fn new(pos: BlockPos, into: Block) -> Self {
        Self { pos, into, cancelled: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    /// What it hardens into.
    pub fn into_block(&self) -> &Block {
        &self.into
    }
}

impl CropTrampleEvent {
    pub fn new(pos: BlockPos) -> Self {
        Self { pos, cancelled: false }
    }

    /// The farmland's.
    pub fn pos(&self) -> BlockPos {
        self.pos
    }
}

impl LeavesDecayEvent {
    pub fn new(pos: BlockPos, leaves: Block) -> Self {
        Self { pos, leaves, cancelled: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    pub fn leaves(&self) -> &Block {
        &self.leaves
    }
}

impl PortalCreateEvent {
    pub fn new(cells: Vec<BlockPos>) -> Self {
        Self { cells, cancelled: false }
    }

    /// The cells that would become portal blocks.
    pub fn cells(&self) -> &[BlockPos] {
        &self.cells
    }
}

impl RedstoneUpdateEvent {
    pub fn new(pos: BlockPos) -> Self {
        Self { pos }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }
}

macro_rules! stoppable {
    ($($event:ty),*) => {$(
        impl $event {
            pub fn cancel(&mut self) {
                self.cancelled = true;
            }

            pub fn is_cancelled(&self) -> bool {
                self.cancelled
            }
        }

        impl Event for $event {
            fn is_stopped(&self) -> bool {
                self.cancelled
            }
        }
    )*};
}

stoppable!(LiquidHardenEvent, CropTrampleEvent, LeavesDecayEvent, PortalCreateEvent);
