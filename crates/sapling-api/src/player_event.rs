//! What a player's own inputs and state changes raise: jumps, swings at nothing, sprint and sneak,
//! the held slot, teleports, game mode and skin. A cancelled one is undone on the player's screen
//! too (docs/research/plugin-api.md).

use crate::{GameMode, Player};

/// Implements [`crate::Event`] and `player()` for player events; with `cancellable`, also
/// `cancel()` and `is_cancelled()` on a `cancelled` field.
macro_rules! player_events {
    (notice: $($notice:ty),*; cancellable: $($event:ty),*) => {
        $(impl $notice {
            /// The player it is about.
            pub fn player(&self) -> &Player {
                &self.player
            }
        }

        impl crate::Event for $notice {
            fn player(&self) -> Option<&Player> {
                Some(&self.player)
            }
        })*
        $(impl $event {
            /// The player it is about.
            pub fn player(&self) -> &Player {
                &self.player
            }

            /// Stops the event: nothing happens, and only `Monitor` listeners still see it.
            pub fn cancel(&mut self) {
                self.cancelled = true;
            }

            pub fn is_cancelled(&self) -> bool {
                self.cancelled
            }
        }

        impl crate::Event for $event {
            fn player(&self) -> Option<&Player> {
                Some(&self.player)
            }

            fn is_stopped(&self) -> bool {
                self.cancelled
            }
        })*
    };
}
pub(crate) use player_events;

/// A player began a jump, as its input says. The client jumps on its own; this cannot stop it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpEvent {
    player: Player,
}

/// A player swung its arm and hit nothing: no block, no entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PunchAirEvent {
    player: Player,
}

/// A player started or stopped sprinting. Cancelled, the player is told it is as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleSprintEvent {
    player: Player,
    sprinting: bool,
    cancelled: bool,
}

/// A player started or stopped sneaking. Cancelled, the player is told it is as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleSneakEvent {
    player: Player,
    sneaking: bool,
    cancelled: bool,
}

/// A player picked another hotbar slot. Cancelled, its hand is put back on `from`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldSlotChangeEvent {
    player: Player,
    from: u8,
    to: u8,
    cancelled: bool,
}

/// The game is about to teleport a player: `/tp`, an ender pearl, chorus fruit. Cancelled, it
/// stays where it is. Teleports a plugin makes are not offered.
#[derive(Debug, Clone, PartialEq)]
pub struct TeleportEvent {
    player: Player,
    from: [f32; 3],
    to: [f32; 3],
    cancelled: bool,
}

/// A player's game mode is about to change: `/gamemode`, its own pause menu, a hardcore death.
/// Cancelled, it keeps the one it has, and its client is told so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameModeChangeEvent {
    player: Player,
    from: GameMode,
    to: GameMode,
    cancelled: bool,
}

/// A player changed its skin in game (the dressing room). Cancelled, nobody sees the new one and
/// the player is shown its old one again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinChangeEvent {
    player: Player,
    skin_id: String,
    cancelled: bool,
}

/// Which portal a player stepped through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Portal {
    /// A nether portal, either way.
    Nether,
    /// An end portal in a stronghold.
    End,
    /// The End's exit portal, home.
    EndExit,
}

/// A player's time in a portal is up and it is about to be sent through. Cancelled, it stays
/// where it is (and is asked again once the portal's cooldown is over).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortalUseEvent {
    player: Player,
    portal: Portal,
    cancelled: bool,
}

player_events!(
    notice: JumpEvent, PunchAirEvent;
    cancellable: ToggleSprintEvent, ToggleSneakEvent, HeldSlotChangeEvent, TeleportEvent, GameModeChangeEvent, SkinChangeEvent, PortalUseEvent
);

impl PortalUseEvent {
    pub fn new(player: Player, portal: Portal) -> Self {
        Self { player, portal, cancelled: false }
    }

    pub fn portal(&self) -> Portal {
        self.portal
    }
}

impl JumpEvent {
    pub fn new(player: Player) -> Self {
        Self { player }
    }
}

impl PunchAirEvent {
    pub fn new(player: Player) -> Self {
        Self { player }
    }
}

impl ToggleSprintEvent {
    pub fn new(player: Player, sprinting: bool) -> Self {
        Self { player, sprinting, cancelled: false }
    }

    /// Whether it is sprinting after the change.
    pub fn sprinting(&self) -> bool {
        self.sprinting
    }
}

impl ToggleSneakEvent {
    pub fn new(player: Player, sneaking: bool) -> Self {
        Self { player, sneaking, cancelled: false }
    }

    /// Whether it is sneaking after the change.
    pub fn sneaking(&self) -> bool {
        self.sneaking
    }
}

impl HeldSlotChangeEvent {
    pub fn new(player: Player, from: u8, to: u8) -> Self {
        Self { player, from, to, cancelled: false }
    }

    pub fn from(&self) -> u8 {
        self.from
    }

    pub fn to(&self) -> u8 {
        self.to
    }
}

impl TeleportEvent {
    pub fn new(player: Player, from: [f32; 3], to: [f32; 3]) -> Self {
        Self { player, from, to, cancelled: false }
    }

    /// Feet before.
    pub fn from(&self) -> [f32; 3] {
        self.from
    }

    /// Feet after.
    pub fn to(&self) -> [f32; 3] {
        self.to
    }
}

impl GameModeChangeEvent {
    pub fn new(player: Player, from: GameMode, to: GameMode) -> Self {
        Self { player, from, to, cancelled: false }
    }

    pub fn from(&self) -> GameMode {
        self.from
    }

    pub fn to(&self) -> GameMode {
        self.to
    }
}

impl SkinChangeEvent {
    pub fn new(player: Player, skin_id: String) -> Self {
        Self { player, skin_id, cancelled: false }
    }

    /// The new skin's id, as the client names it.
    pub fn skin_id(&self) -> &str {
        &self.skin_id
    }
}
