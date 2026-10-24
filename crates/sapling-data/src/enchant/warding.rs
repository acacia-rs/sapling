//! What enchanted armour does for its wearer: the protection enchantments take off a share of a
//! hurt, Fire Protection shortens a burn, Thorns hurts back; and the levels the shard reads off
//! the boots and the helmet (docs/research/enchantments.md, sections 1, 2, 4 and 6).

use acacia_proto::nbt::Value;

use super::{Enchant, level};

/// What the protection enchantments count a hurt as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Harm {
    /// Fire, burning, lava, magma.
    Fire,
    Blast,
    Projectile,
    /// A fall, a stalagmite.
    Fall,
    /// Any other the plain Protection wards.
    Other,
}

/// Points of protection per level of Protection, Fire, Blast, Projectile Protection and Feather
/// Falling; a point takes 4 hundredths off, and no more than `MOST_POINTS` count.
const PER_LEVEL: [u32; 5] = [1, 2, 2, 2, 3];
const MOST_POINTS: u32 = 20;
const POINT: f32 = 0.04;
/// What a level of Fire Protection takes off a burn.
const SHORTENS: f32 = 0.15;
/// Thorns hurts back with this chance a level, for 1 and up to this much more.
const THORNS_CHANCE: f32 = 0.15;
const THORNS_MORE: f32 = 4.0;
/// Thorns above this level hurts back by what is over it.
const THORNS_FLAT: u8 = 10;
/// What a piece with Thorns wears by on a hit, when it hurt back and when it did not.
pub const THORNS_WEAR: [i32; 2] = [3, 1];
/// The most Frost Walker counts.
const FROST_MOST: u8 = 6;

/// What the armour worn wards off.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Warding {
    /// Levels of Protection, Fire, Blast and Projectile Protection and Feather Falling, summed
    /// over the pieces.
    levels: [u32; 5],
    /// The highest of any piece.
    pub respiration: u8,
    pub depth_strider: u8,
    pub frost_walker: u8,
}

impl Warding {
    /// What the armour whose pieces' NBT is `worn`, helmet first, wards off.
    pub fn of(worn: [Option<&Value>; 4]) -> Self {
        let kinds = [Enchant::Protection, Enchant::FireProtection, Enchant::BlastProtection, Enchant::ProjectileProtection, Enchant::FeatherFalling];
        let highest = |enchant: Enchant| worn.iter().map(|tag| level(*tag, enchant)).max().unwrap_or(0);
        Self {
            levels: kinds.map(|enchant| worn.iter().map(|tag| u32::from(level(*tag, enchant))).sum()),
            respiration: highest(Enchant::Respiration),
            depth_strider: highest(Enchant::DepthStrider),
            frost_walker: highest(Enchant::FrostWalker).min(FROST_MOST),
        }
    }

    /// The points of protection against a hurt counted as `harm`, before the cap.
    fn points(&self, harm: Harm) -> u32 {
        let of = |kind: usize| self.levels[kind] * PER_LEVEL[kind];
        of(0) + match harm {
            Harm::Fire => of(1),
            Harm::Blast => of(2),
            Harm::Projectile => of(3),
            Harm::Fall => of(4),
            Harm::Other => 0,
        }
    }

    /// What is left of a hurt of `amount` counted as `harm`.
    pub fn spared(&self, amount: f32, harm: Harm) -> f32 {
        amount * (1.0 - self.points(harm).min(MOST_POINTS) as f32 * POINT)
    }

    /// The ticks a burn of `ticks` lasts.
    pub fn burns_for(&self, ticks: u16) -> u16 {
        let fire = self.levels[1];
        if fire == 0 {
            return ticks;
        }
        (f32::from(ticks) * (1.0 - SHORTENS * fire as f32).clamp(0.0, 1.0)).round() as u16
    }
}

/// What a piece with Thorns at `level` hurts an attacker back by, if it does; `roll` gives
/// numbers in [0, 1).
pub fn thorns(level: u8, roll: &mut dyn FnMut() -> f32) -> Option<f32> {
    if level == 0 || roll() >= THORNS_CHANCE * f32::from(level) {
        return None;
    }
    Some(if level > THORNS_FLAT { f32::from(level - THORNS_FLAT) } else { 1.0 + (roll() * THORNS_MORE).floor() })
}
