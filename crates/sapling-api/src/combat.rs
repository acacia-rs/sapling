//! Harm, death and rising again, as plugins hear them.

use crate::{DamageCause, Event, Player, Target};

/// Who dealt a hit, as far as the game keeps it: a player, or a mob of a kind.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Attacker {
    Player(Player),
    /// Namespaced: `minecraft:zombie`.
    Mob { kind: String },
}

/// A player or a mob is about to be hurt. Cancelled, it takes no harm and is not thrown.
///
/// [`DamageEvent::amount`] is the hit before armour and resistance; for what a player's
/// surroundings do to it (a fall, fire, lava, drowning, suffocation, starving) it is the health
/// it loses in the end. Not offered yet: what their surroundings do to mobs.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageEvent {
    victim: Target,
    cause: DamageCause,
    attacker: Option<Attacker>,
    pub amount: f32,
    cancelled: bool,
}

impl DamageEvent {
    pub fn new(victim: Target, cause: DamageCause, attacker: Option<Attacker>, amount: f32) -> Self {
        Self { victim, cause, attacker, amount, cancelled: false }
    }

    pub fn victim(&self) -> &Target {
        &self.victim
    }

    pub fn cause(&self) -> DamageCause {
        self.cause
    }

    pub fn attacker(&self) -> Option<&Attacker> {
        self.attacker.as_ref()
    }

    /// Stops the hit: no harm, no throw, and only `Monitor` listeners still see it.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

impl Event for DamageEvent {
    fn player(&self) -> Option<&Player> {
        match &self.victim {
            Target::Player(player) => Some(player),
            Target::Entity(_) => None,
        }
    }

    fn is_stopped(&self) -> bool {
        self.cancelled
    }
}

/// What the world is told of a death.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum DeathMessage {
    /// The game's own words: `death.attack.player` and the like.
    #[default]
    Vanilla,
    Hidden,
    Text(String),
}

/// A player died. It cannot be stopped (cancel the [`DamageEvent`] instead), but listeners decide
/// what the player keeps and what the world is told; the death screen keeps the game's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeathEvent {
    player: Player,
    cause: DamageCause,
    attacker: Option<Attacker>,
    /// Keep items and experience rather than drop them; starts as the world's `keepInventory`.
    pub keep_inventory: bool,
    pub message: DeathMessage,
}

impl DeathEvent {
    pub fn new(player: Player, cause: DamageCause, attacker: Option<Attacker>, keep_inventory: bool) -> Self {
        Self { player, cause, attacker, keep_inventory, message: DeathMessage::Vanilla }
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn cause(&self) -> DamageCause {
        self.cause
    }

    pub fn attacker(&self) -> Option<&Attacker> {
        self.attacker.as_ref()
    }
}

impl Event for DeathEvent {
    fn player(&self) -> Option<&Player> {
        Some(&self.player)
    }
}

/// A dead player rises: at [`RespawnEvent::feet`], which starts as its bed or its world's spawn,
/// in the world it died in.
#[derive(Debug, Clone, PartialEq)]
pub struct RespawnEvent {
    player: Player,
    pub feet: [f32; 3],
}

impl RespawnEvent {
    pub fn new(player: Player, feet: [f32; 3]) -> Self {
        Self { player, feet }
    }

    pub fn player(&self) -> &Player {
        &self.player
    }
}

impl Event for RespawnEvent {
    fn player(&self) -> Option<&Player> {
        Some(&self.player)
    }
}
