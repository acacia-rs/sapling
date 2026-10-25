//! Enchantments: their ids in a stack's `ench` list, and what each level does, as BDS reckons it
//! (docs/research/enchantments.md). Protection and the rest of what armour's do: warding.rs. The
//! workstations that give and take them (docs/research/workstations.md): the enchanting table
//! (table.rs), the anvil (anvil.rs) and the grindstone (grind.rs), all by kinds.rs.

pub mod anvil;
pub mod grind;
mod kinds;
mod list;
pub mod table;
mod warding;

pub use list::{all, with_all};
pub use table::Offer;
pub use warding::{Harm, THORNS_WEAR, Warding, thorns};

/// The random numbers the workstations draw: BDS's `Random` in the shard.
pub trait Draws {
    /// From 0 to below `bound`; 0 for a bound below 1.
    fn int(&mut self, bound: i32) -> i32;
    /// From 0 to below 1.
    fn float(&mut self) -> f32;
}

use acacia_proto::nbt::Value;

/// The enchantments sapling knows, numbered as a stack's `ench` list numbers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Enchant {
    Protection = 0,
    FireProtection = 1,
    FeatherFalling = 2,
    BlastProtection = 3,
    ProjectileProtection = 4,
    Thorns = 5,
    Respiration = 6,
    DepthStrider = 7,
    AquaAffinity = 8,
    Sharpness = 9,
    Smite = 10,
    BaneOfArthropods = 11,
    Knockback = 12,
    FireAspect = 13,
    Looting = 14,
    Efficiency = 15,
    SilkTouch = 16,
    Unbreaking = 17,
    Fortune = 18,
    FrostWalker = 25,
    Mending = 26,
    Binding = 27,
    Vanishing = 28,
}

/// The level of `enchant` on a stack whose NBT is `tag`; 0 without it.
pub fn level(tag: Option<&Value>, enchant: Enchant) -> u8 {
    let Some(Value::List(list)) = tag.and_then(|tag| tag.get("ench")) else { return 0 };
    let found = list.items.iter().find(|entry| entry.get("id") == Some(&Value::Short(enchant as i16)));
    match found.and_then(|entry| entry.get("lvl")) {
        Some(Value::Short(level)) => (*level).clamp(0, i16::from(u8::MAX)) as u8,
        _ => 0,
    }
}

/// What Efficiency adds to the speed of a tool on what it is made for.
pub fn efficiency(level: u8) -> f32 {
    if level == 0 { 0.0 } else { f32::from(level).powi(2) + 1.0 }
}

/// The chance in a hundred that a use wears a stack with Unbreaking at `level`, `armour` or a tool.
// TODO: armour's 60 + 40 / (level + 1) is the wiki's; only the tools' was decompiled (block-rules.md).
pub fn wear_chance(level: u8, armour: bool) -> u32 {
    let level = u32::from(level);
    match (level, armour) {
        (0, _) => 100,
        (_, true) => 60 + 40 / (level + 1),
        (_, false) => 100 / (level + 1),
    }
}

/// What Sharpness, Smite and Bane of Arthropods on `weapon` add to a hit on a mob of `target`
/// (namespaced), or on a player for `None`: whole points, the sum floored, so Sharpness I-V adds
/// 1, 2, 3, 5, 6.
pub fn melee_bonus(weapon: Option<&Value>, target: Option<&str>) -> f32 {
    let at = |enchant: Enchant| f32::from(level(weapon, enchant));
    let sharp = at(Enchant::Sharpness) * SHARPNESS;
    let smite = if target.is_some_and(crate::is_undead) { at(Enchant::Smite) * SMITE } else { 0.0 };
    let bane = if target.is_some_and(crate::is_arthropod) { at(Enchant::BaneOfArthropods) * BANE } else { 0.0 };
    (sharp + smite + bane).floor()
}

const SHARPNESS: f32 = 1.25;
const SMITE: f32 = 2.5;
const BANE: f32 = 2.5;
/// Bane of Arthropods slows at this amplifier (BDS's; the Java game's is 3) for a second and up
/// to half a second a level more.
pub const BANE_SLOWNESS: u8 = 0;
const BANE_TICKS: u32 = 20;
const BANE_MORE: u32 = 10;
const ASPECT_TICKS: u16 = 80;
// TODO: BDS reckons 1.15 a level and how it throws by it was not traced; the Java game's half a block.
const KNOCKBACK: f32 = 0.5;

/// Ticks of slowness Bane of Arthropods at `level` gives; `roll` gives numbers in [0, 1).
pub fn bane_slowness(level: u8, roll: &mut dyn FnMut() -> f32) -> u32 {
    BANE_TICKS + (roll() * (BANE_MORE * u32::from(level)) as f32) as u32
}

/// Ticks Fire Aspect at `level` sets what it hits alight for.
pub fn aspect_burn(level: u8) -> u16 {
    ASPECT_TICKS * u16::from(level)
}

/// The speed away and up a hit throws at with Knockback at `level`, from what it throws at without.
pub fn knockback(level: u8, [along, up]: [f32; 2]) -> [f32; 2] {
    [along + KNOCKBACK * f32::from(level), up]
}

/// Mending with an orb worth `value` on a stack worn by `damage`: what it mends, and what is left
/// of the value.
pub fn mend(damage: i32, value: i32) -> (i32, i32) {
    let mended = damage.min(value * 2);
    (mended, value - mended / 2)
}

/// The chance that a tick out of breath costs none of it with Respiration at `level`.
pub fn holds_breath(level: u8) -> f32 {
    f32::from(level) / (f32::from(level) + 1.0)
}

/// How far around, across a square, Frost Walker at `level` (at most 6 counts: `Warding`)
/// freezes water; 0 without it.
pub fn frost_reach(level: u8) -> i32 {
    if level == 0 { 0 } else { 2 + i32::from(level) }
}

/// What a loot table's `looting_enchant` of `[min, max]` adds at `level`.
pub fn looted([min, max]: [f32; 2], level: u8, roll: &mut dyn FnMut() -> f32) -> u32 {
    ((min + roll() * (max - min)) * f32::from(level)).round().max(0.0) as u32
}

/// The chance of a loot table's `random_chance_with_looting` at `level`.
pub fn looting_chance(chance: f32, per_level: f32, level: u8) -> f32 {
    chance + per_level * f32::from(level)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod workstation_tests;
