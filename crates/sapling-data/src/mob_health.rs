//! What tells viewers of a mob's health, breath and being on fire, in BDS's shapes
//! (research/captures/bds-burn.txt, bds-harm.txt), and what fire, falls and water do nothing to (`data/mob-traits.tsv`,
//! README "Mob traits").

use acacia_proto::packets::{SetEntityData, UpdateAttributes};
use acacia_proto::types::{
    EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1,
    PlayerAttributesItem,
};

use crate::entity::mob_flags;

const HEALTH: &str = "minecraft:health";

/// What does not hurt a kind of mob.
// TODO: `no_air`, a fish out of water, is in the table and not read.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MobTraits {
    pub fire_immune: bool,
    /// Daylight under an open sky sets it on fire.
    pub burns_in_sun: bool,
    pub no_fall: bool,
    pub breathes_water: bool,
    /// It teleports about by itself (its `minecraft:teleport`'s `random_teleports`).
    pub teleports: bool,
    /// The least and most experience a grown one killed by a player leaves.
    pub experience: Option<(i32, i32)>,
    pub hunts: Option<Hunts>,
    /// The speed a kind that walks strolls at: its `minecraft:movement` times its
    /// `random_stroll`'s `speed_multiplier`.
    pub strolls: Option<f32>,
    /// The speed it flees a hit at: its movement times its `panic`'s `speed_multiplier`.
    pub panics: Option<f32>,
}

/// A kind that hunts players (README "Mob traits").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hunts {
    /// What its hit costs on normal difficulty, with what it holds.
    pub attack: f32,
    /// Its `minecraft:movement` times its attack's `speed_multiplier`.
    pub speed: f32,
    /// How far off it takes a player as its prey.
    pub sees: f32,
    /// The effect its hit gives (`sapling_api::Effect::name`) and for how many ticks.
    pub gives: Option<(&'static str, u32)>,
    /// The power it blows up with near its prey, where it hits nobody.
    pub explodes: Option<f32>,
    /// The ticks between two arrows it looses at its prey, where it hits nobody.
    pub shoots: Option<u32>,
    /// It hunts nobody but a player that hit it.
    pub provoked: bool,
    /// An archer that walks on at its prey while it shoots.
    pub closes: bool,
    /// It fights its prey its own way and never walks at it: `spells` (the evoker), `leaps` (the
    /// breeze), `fireballs` (the ghast).
    pub fights: Option<&'static str>,
    /// Its hit costs a whole number from `attack` up to this, where it varies, and it throws
    /// harder than others (the iron golem).
    pub upto: Option<f32>,
    /// The ticks between two of its hits, where they are not the 20 of most.
    pub every: Option<u32>,
    /// It takes no prey while the sun shines on it (the spiders).
    pub by_dark: bool,
    /// The ticks from its taking its prey to its first shot, where they are not `shoots`.
    pub aims: Option<u32>,
    /// It shoots this many times, this many ticks apart, each time it shoots.
    pub volley: Option<(u32, u32)>,
    /// What it shoots, where it is not an arrow: `small_fireball`, `llama_spit`.
    pub shot: Option<&'static str>,
    /// It lets its prey go this many ticks after taking it (a llama's anger).
    pub calms: Option<u32>,
    /// It lets its prey go once it has hit it (a panda's, a bee's `attack_once`).
    pub once: bool,
}

pub fn mob_traits(identifier: &str) -> MobTraits {
    let row = include_str!("../data/mob-traits.tsv").lines().filter_map(|row| row.split_once('\t')).find(|(name, _)| *name == identifier);
    let has = |trait_: &str| row.is_some_and(|(_, traits)| traits.split(' ').any(|held| held == trait_));
    let range = |range: &str| range.split_once('-').and_then(|(least, most)| Some((least.parse().ok()?, most.parse().ok()?)));
    let experience = row.and_then(|(_, traits)| traits.split(' ').find_map(|held| held.strip_prefix("xp="))).and_then(range);
    let value = |name: &str| row.and_then(|(_, traits)| traits.split(' ').find_map(|held| held.strip_prefix(name)));
    let number = |name: &str| value(name).and_then(|value| value.parse::<f32>().ok());
    let gives = value("hits=").and_then(|given| given.split_once(':')).and_then(|(effect, ticks)| Some((effect, ticks.parse().ok()?)));
    let hunts = number("attack=").zip(number("speed=")).zip(number("sees=")).map(|((attack, speed), sees)| Hunts { attack, speed, sees, gives, explodes: number("explodes="), shoots: number("shoots=").map(|ticks| ticks as u32), provoked: has("provoked"), closes: has("closes"), fights: value("fights="), upto: number("upto="), every: number("every=").map(|ticks| ticks as u32), by_dark: has("by_dark"), aims: number("aims=").map(|ticks| ticks as u32), volley: value("volley=").and_then(|volley| volley.split_once('x')).and_then(|(shots, gap)| Some((shots.parse().ok()?, gap.parse().ok()?))), shot: value("shot="), calms: number("calms=").map(|ticks| ticks as u32), once: has("once") });
    MobTraits { fire_immune: has("fire_immune"), burns_in_sun: has("burns_in_sun"), no_fall: has("no_fall"), breathes_water: has("breathes_water"), teleports: has("teleports"), experience, hunts, strolls: number("strolls="), panics: number("panics=") }
}

/// The mob `id` has `left` of `max` health.
pub fn mob_health_update(id: u64, left: f32, max: f32) -> UpdateAttributes {
    let health = PlayerAttributesItem { min: 0.0, max, current: left, default_min: 0.0, default_max: max, default: max, name: HEALTH.to_owned(), modifiers: Vec::new() };
    UpdateAttributes { runtime_entity_id: id, attributes: vec![health], tick: 0 }
}

/// The flags of the mob `id`, a grown one of `identifier`, on fire or not.
pub fn mob_fire_flags(id: u64, identifier: &str, on_fire: bool) -> SetEntityData {
    let flags = MetadataFlags1(mob_flags(identifier).0 | if on_fire { MetadataFlags1::ONFIRE.0 } else { 0 });
    let item = MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) };
    SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// The mob `id`, a grown one of `identifier`, has a lit fuse with `left` ticks to go.
pub fn mob_fuse(id: u64, identifier: &str, left: i32) -> SetEntityData {
    let flags = MetadataFlags1(mob_flags(identifier).0 | MetadataFlags1::IGNITED.0);
    let metadata = vec![
        MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) },
        MetadataDictionaryItem { key: Key::FuseLength, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(left)) },
    ];
    SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// The mob `id`, a grown one of `identifier`, casts a spell of `colour` (ARGB), or stops casting.
pub fn mob_spell(id: u64, identifier: &str, colour: Option<i32>) -> SetEntityData {
    let flags = MetadataFlags1(mob_flags(identifier).0 | if colour.is_some() { MetadataFlags1::EVOKER_SPELL.0 } else { 0 });
    let mut metadata = vec![MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) }];
    // BDS never clears the colour; it rewrites it with the next spell.
    if let Some(colour) = colour {
        metadata.push(MetadataDictionaryItem { key: Key::EvokerSpellCastingColor, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(colour)) });
    }
    SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// The mob `id`, a grown one of `identifier`, readies a shot (a ghast's open mouth) or stops.
pub fn mob_charge(id: u64, identifier: &str, charged: bool) -> SetEntityData {
    let flags = MetadataFlags1(mob_flags(identifier).0 | if charged { MetadataFlags1::CHARGED.0 } else { 0 });
    let metadata = vec![
        MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) },
        MetadataDictionaryItem { key: Key::ChargeAmount, r#type: Type::Byte, legacy_type: 0, value: Value::Default(Plain::Byte(i8::from(charged))) },
    ];
    SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// The mob `id` has `air` ticks of breath left.
pub fn mob_air(id: u64, air: i16) -> SetEntityData {
    let item = MetadataDictionaryItem { key: Key::Air, r#type: Type::Short, legacy_type: 1, value: Value::Default(Plain::Short(air)) };
    SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_spares_a_strider_a_fall_a_chicken_and_water_a_cod() {
        assert_eq!(mob_traits("minecraft:pig"), MobTraits { experience: Some((1, 3)), strolls: Some(0.25), panics: Some(0.3125), ..MobTraits::default() });
        assert_eq!((mob_traits("minecraft:creeper").experience, mob_traits("minecraft:armor_stand").experience), (Some((5, 5)), None));
        assert!(mob_traits("minecraft:strider").fire_immune && mob_traits("minecraft:chicken").no_fall && mob_traits("minecraft:cod").breathes_water);
        assert!(!mob_traits("minecraft:chicken").fire_immune);
    }

    #[test]
    fn a_burning_pig_keeps_its_own_flags() {
        let flags = |on_fire| match mob_fire_flags(7, "minecraft:pig", on_fire).metadata[0].value {
            Value::Flags(MetadataFlags1(flags)) => flags,
            _ => unreachable!(),
        };
        assert_eq!(flags(true), flags(false) | 1);
        assert_ne!(flags(false), 0);
        assert_eq!(mob_health_update(7, 3.0, 10.0).attributes[0].current, 3.0);
    }
}
