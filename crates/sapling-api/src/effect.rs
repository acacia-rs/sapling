//! Effects on players: the ones BDS was drilled on (docs/research/effects.md).

use crate::{PlayerId, ShardCtx};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Speed,
    Slowness,
    /// Told to the client, which digs faster; the shard does nothing more with it.
    Haste,
    /// Told to the client alone, as haste.
    MiningFatigue,
    Strength,
    InstantHealth,
    InstantDamage,
    JumpBoost,
    /// Told to the client, which sways; the shard does nothing more with it.
    Nausea,
    Regeneration,
    Resistance,
    FireResistance,
    WaterBreathing,
    /// Told to the client alone, as are blindness, night vision and levitation.
    Invisibility,
    Blindness,
    NightVision,
    Hunger,
    Weakness,
    Poison,
    Wither,
    HealthBoost,
    Absorption,
    Saturation,
    Levitation,
    SlowFalling,
}

/// Each effect with the name commands know it by, the translation key of its name, and the id
/// the wire knows it by.
const KNOWN: [(Effect, &str, &str, i32); 25] = [
    (Effect::Speed, "speed", "potion.moveSpeed", 1),
    (Effect::Slowness, "slowness", "potion.moveSlowdown", 2),
    (Effect::Haste, "haste", "potion.digSpeed", 3),
    (Effect::MiningFatigue, "mining_fatigue", "potion.digSlowDown", 4),
    (Effect::Strength, "strength", "potion.damageBoost", 5),
    (Effect::InstantHealth, "instant_health", "potion.heal", 6),
    (Effect::InstantDamage, "instant_damage", "potion.harm", 7),
    (Effect::JumpBoost, "jump_boost", "potion.jump", 8),
    (Effect::Nausea, "nausea", "potion.confusion", 9),
    (Effect::Regeneration, "regeneration", "potion.regeneration", 10),
    (Effect::Resistance, "resistance", "potion.resistance", 11),
    (Effect::FireResistance, "fire_resistance", "potion.fireResistance", 12),
    (Effect::WaterBreathing, "water_breathing", "potion.waterBreathing", 13),
    (Effect::Invisibility, "invisibility", "potion.invisibility", 14),
    (Effect::Blindness, "blindness", "potion.blindness", 15),
    (Effect::NightVision, "night_vision", "potion.nightVision", 16),
    (Effect::Hunger, "hunger", "potion.hunger", 17),
    (Effect::Weakness, "weakness", "potion.weakness", 18),
    (Effect::Poison, "poison", "potion.poison", 19),
    (Effect::Wither, "wither", "potion.wither", 20),
    (Effect::HealthBoost, "health_boost", "potion.healthBoost", 21),
    (Effect::Absorption, "absorption", "potion.absorption", 22),
    (Effect::Saturation, "saturation", "potion.saturation", 23),
    (Effect::Levitation, "levitation", "potion.levitation", 24),
    (Effect::SlowFalling, "slow_falling", "potion.slowFalling", 27),
];

impl Effect {
    pub const ALL: [Effect; 25] = {
        let mut all = [Effect::Speed; 25];
        let mut at = 0;
        while at < all.len() {
            all[at] = KNOWN[at].0;
            at += 1;
        }
        all
    };

    fn known(self) -> &'static (Effect, &'static str, &'static str, i32) {
        KNOWN.iter().find(|known| known.0 == self).expect("every effect is in the table")
    }

    /// The name commands know it by.
    pub fn name(self) -> &'static str {
        self.known().1
    }

    /// The translation key of its name.
    pub fn key(self) -> &'static str {
        self.known().2
    }

    /// The id the wire knows it by.
    pub fn id(self) -> i32 {
        self.known().3
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_effect_has_a_name_and_an_id_of_its_own() {
        for (at, effect) in Effect::ALL.into_iter().enumerate() {
            assert!(Effect::ALL[..at].iter().all(|other| other.id() != effect.id() && other.name() != effect.name()), "{effect:?}");
        }
        assert_eq!((Effect::SlowFalling.id(), Effect::Weakness.name(), Effect::Strength.key()), (27, "weakness", "potion.damageBoost"));
    }
}
