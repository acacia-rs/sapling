//! Effects on players: the ones BDS was drilled on (docs/research/effects.md).

use crate::{PlayerId, ShardCtx};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Speed,
    InstantHealth,
    InstantDamage,
    Regeneration,
    Hunger,
    Poison,
    Wither,
    Absorption,
    Saturation,
}

impl Effect {
    pub const ALL: [Effect; 9] = [Self::Speed, Self::InstantHealth, Self::InstantDamage, Self::Regeneration, Self::Hunger, Self::Poison, Self::Wither, Self::Absorption, Self::Saturation];

    /// The name commands know it by.
    pub fn name(self) -> &'static str {
        match self {
            Self::Speed => "speed",
            Self::InstantHealth => "instant_health",
            Self::InstantDamage => "instant_damage",
            Self::Regeneration => "regeneration",
            Self::Hunger => "hunger",
            Self::Poison => "poison",
            Self::Wither => "wither",
            Self::Absorption => "absorption",
            Self::Saturation => "saturation",
        }
    }

    /// The translation key of its name.
    pub fn key(self) -> &'static str {
        match self {
            Self::Speed => "potion.moveSpeed",
            Self::InstantHealth => "potion.heal",
            Self::InstantDamage => "potion.harm",
            Self::Regeneration => "potion.regeneration",
            Self::Hunger => "potion.hunger",
            Self::Poison => "potion.poison",
            Self::Wither => "potion.wither",
            Self::Absorption => "potion.absorption",
            Self::Saturation => "potion.saturation",
        }
    }

    /// The id the wire knows it by.
    pub fn id(self) -> i32 {
        match self {
            Self::Speed => 1,
            Self::InstantHealth => 6,
            Self::InstantDamage => 7,
            Self::Regeneration => 10,
            Self::Hunger => 17,
            Self::Poison => 19,
            Self::Wither => 20,
            Self::Absorption => 22,
            Self::Saturation => 23,
        }
    }

    /// Done in a tick, however long it was given for.
    pub fn is_instant(self) -> bool {
        matches!(self, Self::InstantHealth | Self::InstantDamage | Self::Saturation)
    }
}

impl ShardCtx<'_> {
    /// Puts `effect` on a player for `ticks` at `amplifier` (0 is the first level), shown with
    /// particles or not. One already on the player stays if it is stronger, or as strong and
    /// longer. False for a player who is not in the world.
    pub fn give_effect(&mut self, player: PlayerId, effect: Effect, ticks: u32, amplifier: u8, particles: bool) -> bool {
        self.backend.give_effect(player, effect, ticks, amplifier, particles)
    }

    /// Takes `effect` off a player; false where it was not on it.
    pub fn remove_effect(&mut self, player: PlayerId, effect: Effect) -> bool {
        self.backend.remove_effect(player, effect)
    }

    /// Takes every effect off a player; false where it had none.
    pub fn clear_effects(&mut self, player: PlayerId) -> bool {
        self.backend.clear_effects(player)
    }
}

/// The effects side of [`crate::Backend`].
pub trait EffectBackend {
    fn give_effect(&mut self, player: PlayerId, effect: Effect, ticks: u32, amplifier: u8, particles: bool) -> bool;
    fn remove_effect(&mut self, player: PlayerId, effect: Effect) -> bool;
    fn clear_effects(&mut self, player: PlayerId) -> bool;
}
