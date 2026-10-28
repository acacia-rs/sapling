//! What a player's body and hands do that plugins may stop: picking up, eating, experience, hunger
//! and healing, sleeping, writing on signs, opening containers.

use crate::player_event::player_events;
use crate::{BlockPos, Item, Player};

/// A player is about to pick up an item lying in the world. Cancelled, it stays where it lies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemPickupEvent {
    player: Player,
    item: Item,
    cancelled: bool,
}

/// A player finished eating or drinking what it holds. Cancelled, nothing is eaten: the item
/// stays in its hand and does nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemConsumeEvent {
    player: Player,
    item: Option<Item>,
    cancelled: bool,
}

/// A player is about to gain experience points: an orb, `/xp`, a smelted item. Listeners may
/// change [`ExperienceGainEvent::amount`]; cancelled, it gains none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceGainEvent {
    player: Player,
    pub amount: i32,
    cancelled: bool,
}

/// A player's hunger bar is about to drop, its exhaustion spent. Cancelled, it keeps its hunger.
#[derive(Debug, Clone, PartialEq)]
pub struct FoodLossEvent {
    player: Player,
    from: f32,
    to: f32,
    cancelled: bool,
}

/// Where a player's health comes back from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HealSource {
    /// A full hunger bar mending a wounded player.
    Natural,
    /// The regeneration effect.
    Regeneration,
    /// The instant health effect: a potion, a splash.
    InstantHealth,
}

/// A player is about to heal. Listeners may change [`HealEvent::amount`]; cancelled, it heals
/// nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct HealEvent {
    player: Player,
    pub amount: f32,
    source: HealSource,
    cancelled: bool,
}

/// A player is about to lie down in a bed, every check passed. Cancelled, it stays up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SleepEvent {
    player: Player,
    bed: BlockPos,
    cancelled: bool,
}

/// A player wrote on a sign. Cancelled, the sign keeps its text and the player is shown it again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignEditEvent {
    player: Player,
    pos: BlockPos,
    front: bool,
    old: String,
    new: String,
    cancelled: bool,
}

/// A player is about to open the screen of a block: a chest, a furnace, a crafting table, an
/// anvil. Cancelled, no screen opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerOpenEvent {
    player: Player,
    pos: BlockPos,
    block: String,
    cancelled: bool,
}

/// A player's block screen closed: it closed it, opened another, walked off or the block went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerCloseEvent {
    player: Player,
    pos: BlockPos,
}

player_events!(notice: ContainerCloseEvent; cancellable: ItemPickupEvent, ItemConsumeEvent, ExperienceGainEvent, FoodLossEvent, HealEvent, SleepEvent, SignEditEvent, ContainerOpenEvent);

impl ItemPickupEvent {
    pub fn new(player: Player, item: Item) -> Self {
        Self { player, item, cancelled: false }
    }

    pub fn item(&self) -> &Item {
        &self.item
    }
}

impl ItemConsumeEvent {
    pub fn new(player: Player, item: Option<Item>) -> Self {
        Self { player, item, cancelled: false }
    }

    /// What it ate or drank.
    pub fn item(&self) -> Option<&Item> {
        self.item.as_ref()
    }
}

impl ExperienceGainEvent {
    pub fn new(player: Player, amount: i32) -> Self {
        Self { player, amount, cancelled: false }
    }
}

impl FoodLossEvent {
    pub fn new(player: Player, from: f32, to: f32) -> Self {
        Self { player, from, to, cancelled: false }
    }

    /// The hunger before, 0 to 20.
    pub fn from(&self) -> f32 {
        self.from
    }

    pub fn to(&self) -> f32 {
        self.to
    }
}

impl HealEvent {
    pub fn new(player: Player, amount: f32, source: HealSource) -> Self {
        Self { player, amount, source, cancelled: false }
    }

    pub fn source(&self) -> HealSource {
        self.source
    }
}

impl SleepEvent {
    pub fn new(player: Player, bed: BlockPos) -> Self {
        Self { player, bed, cancelled: false }
    }

    /// The bed's head.
    pub fn bed(&self) -> BlockPos {
        self.bed
    }
}

impl SignEditEvent {
    pub fn new(player: Player, pos: BlockPos, front: bool, old: String, new: String) -> Self {
        Self { player, pos, front, old, new, cancelled: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    /// Whether the front side was written on, not the back.
    pub fn front(&self) -> bool {
        self.front
    }

    pub fn old_text(&self) -> &str {
        &self.old
    }

    pub fn new_text(&self) -> &str {
        &self.new
    }
}

impl ContainerOpenEvent {
    pub fn new(player: Player, pos: BlockPos, block: String) -> Self {
        Self { player, pos, block, cancelled: false }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }

    /// Namespaced: `minecraft:chest`.
    pub fn block(&self) -> &str {
        &self.block
    }
}

impl ContainerCloseEvent {
    pub fn new(player: Player, pos: BlockPos) -> Self {
        Self { player, pos }
    }

    pub fn pos(&self) -> BlockPos {
        self.pos
    }
}
