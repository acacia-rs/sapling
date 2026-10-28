//! What a player's hands and screens do past the life events: starting a break, picking a block,
//! letting go of a drawn bow, wearing a tool, moving stacks in a container, turning a lectern's
//! page, catching fire, punching a fire out, and the client's own diagnostics. A cancelled one is
//! undone on the player's screen too (docs/research/ui-events.md).

use crate::player_event::player_events;
use crate::{Block, BlockPos, Item, Player};

/// A player started breaking a block (its first hit). Cancelled, the break is not timed and the
/// crack is stopped on its screen, so finishing it is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartBreakEvent {
    player: Player,
    pos: BlockPos,
    block: Block,
    cancelled: bool,
}

/// A creative player picked a block (middle click). Cancelled, nothing comes to its hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockPickEvent {
    player: Player,
    pos: BlockPos,
    block: Block,
    cancelled: bool,
}

/// A player let go of a bow, crossbow or trident it was drawing. Cancelled, nothing is loosed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemReleaseEvent {
    player: Player,
    item: Item,
    ticks: u64,
    cancelled: bool,
}

/// A tool, weapon or armour piece of a player's is about to wear by [`ItemDamageEvent::damage`]
/// uses, which listeners may change. Cancelled, it does not wear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDamageEvent {
    player: Player,
    item: Item,
    pub damage: i32,
    cancelled: bool,
}

/// What a click did to a slot of the container a player has open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SlotChange {
    /// Some or all of what the slot held left it.
    Take,
    /// The slot holds more than it did.
    Place,
    /// What left the slot was thrown out of the screen.
    Drop,
}

/// A player's inventory request is about to change a slot of the container it has open; one
/// event per slot changed. Cancelled, the whole request is refused and the client shown its
/// windows as they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerSlotEvent {
    player: Player,
    pos: Option<BlockPos>,
    slot: usize,
    change: SlotChange,
    item: Item,
    cancelled: bool,
}

/// A player is turning the page of a lectern's book. Listeners may change
/// [`LecternPageTurnEvent::to`]; cancelled, the page stays and the client is shown it again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LecternPageTurnEvent {
    player: Player,
    pos: BlockPos,
    from: u32,
    pub to: u32,
    cancelled: bool,
}

/// A player caught fire (it was not burning). Cancelled, it is put out at once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetOnFireEvent {
    player: Player,
    ticks: u16,
    cancelled: bool,
}

/// A player is about to punch out the fire at `pos`. Cancelled, the fire stays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FireExtinguishEvent {
    player: Player,
    pos: BlockPos,
    cancelled: bool,
}

/// Times a client measured of itself, as it reports them when its diagnostics are on.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Diagnostics {
    pub frames_per_second: f32,
    /// Milliseconds.
    pub server_sim_tick_time: f32,
    pub client_sim_tick_time: f32,
    pub begin_frame_time: f32,
    pub input_time: f32,
    pub render_time: f32,
    pub end_frame_time: f32,
    pub remainder_time_percent: f32,
    pub unaccounted_time_percent: f32,
}

/// A client sent its diagnostics (ServerboundDiagnostics).
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticsEvent {
    player: Player,
    diagnostics: Diagnostics,
}

player_events!(notice: DiagnosticsEvent; cancellable: StartBreakEvent, BlockPickEvent, ItemReleaseEvent, ItemDamageEvent, ContainerSlotEvent, LecternPageTurnEvent, SetOnFireEvent, FireExtinguishEvent);

impl StartBreakEvent {
    pub fn new(player: Player, pos: BlockPos, block: Block) -> Self {
        Self { player, pos, block, cancelled: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    pub fn block(&self) -> &Block {
        &self.block
    }
}

impl BlockPickEvent {
    pub fn new(player: Player, pos: BlockPos, block: Block) -> Self {
        Self { player, pos, block, cancelled: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    pub fn block(&self) -> &Block {
        &self.block
    }
}

impl ItemReleaseEvent {
    pub fn new(player: Player, item: Item, ticks: u64) -> Self {
        Self { player, item, ticks, cancelled: false }
    }

    pub fn item(&self) -> &Item {
        &self.item
    }

    /// How long it was drawn.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }
}

impl ItemDamageEvent {
    pub fn new(player: Player, item: Item, damage: i32) -> Self {
        Self { player, item, damage, cancelled: false }
    }

    pub fn item(&self) -> &Item {
        &self.item
    }
}

impl ContainerSlotEvent {
    pub fn new(player: Player, pos: Option<BlockPos>, slot: usize, change: SlotChange, item: Item) -> Self {
        Self { player, pos, slot, change, item, cancelled: false }
    }

    /// The container block; `None` for one that is no block (an ender chest's own slots are
    /// the player's, a boat's chest).
    pub fn pos(&self) -> Option<BlockPos> {
        self.pos
    }

    pub fn slot(&self) -> usize {
        self.slot
    }

    pub fn change(&self) -> SlotChange {
        self.change
    }

    /// What was taken, placed or dropped: the difference, not the whole stack.
    pub fn item(&self) -> &Item {
        &self.item
    }
}

impl LecternPageTurnEvent {
    pub fn new(player: Player, pos: BlockPos, from: u32, to: u32) -> Self {
        Self { player, pos, from, to, cancelled: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    pub fn from(&self) -> u32 {
        self.from
    }
}

impl SetOnFireEvent {
    pub fn new(player: Player, ticks: u16) -> Self {
        Self { player, ticks, cancelled: false }
    }

    /// How long it would burn.
    pub fn ticks(&self) -> u16 {
        self.ticks
    }
}

impl FireExtinguishEvent {
    pub fn new(player: Player, pos: BlockPos) -> Self {
        Self { player, pos, cancelled: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }
}

impl DiagnosticsEvent {
    pub fn new(player: Player, diagnostics: Diagnostics) -> Self {
        Self { player, diagnostics }
    }

    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }
}
