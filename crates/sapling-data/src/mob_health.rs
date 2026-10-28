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
    /// The ticks of breath it holds, where they are more than the usual 300 (a dolphin's 4800).
    pub breath: Option<u32>,
    /// It teleports about by itself (its `minecraft:teleport`'s `random_teleports`).
    pub teleports: bool,
    /// It swims up in water (`minecraft:behavior.float`).
    pub floats: bool,
    /// No hit throws it (`minecraft:knockback_resistance` 1, the warden's).
    pub steadfast: bool,
    /// Its `minecraft:knockback_resistance`: the share of a throw it does not take (1 where steadfast).
    pub resists: f32,
    /// Nothing pushes it aside: it has no `minecraft:pushable_by_entity` (nor an older pushable
    /// `minecraft:pushable`) in any of its groups.
    pub unpushable: bool,
    /// The least and most experience a grown one killed by a player leaves.
    pub experience: Option<(i32, i32)>,
    pub hunts: Option<Hunts>,
    /// The speed a kind that walks strolls at: its `minecraft:movement` times its
    /// `random_stroll`'s `speed_multiplier`.
    pub strolls: Option<f32>,
    /// The speed it flees a hit at: its movement times its `panic`'s `speed_multiplier`.
    pub panics: Option<f32>,
    /// The speed a kind that swims about swims at: its `underwater_movement` times its
    /// `random_swim`'s `speed_multiplier`, and how far across and up or down a swim goes.
    pub swims: Option<(f32, [u32; 2])>,
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
    /// breeze), `fireballs` (the ghast), `hops` (slimes and magma cubes).
    pub fights: Option<&'static str>,
    /// Its hit costs a whole number from `attack` up to this, where it varies, and it throws
    /// harder than others (the iron golem).
    pub upto: Option<f32>,
    /// Its hit costs anything from `attack` up to this, fractions too (the hoglin and the zoglin:
    /// research/captures/bds-wander-findings.md).
    pub varies: Option<f32>,
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
    let hunts = number("attack=").zip(number("speed=")).zip(number("sees=")).map(|((attack, speed), sees)| Hunts { attack, speed, sees, gives, explodes: number("explodes="), shoots: number("shoots=").map(|ticks| ticks as u32), provoked: has("provoked"), closes: has("closes"), fights: value("fights="), upto: number("upto="), varies: number("varies="), every: number("every=").map(|ticks| ticks as u32), by_dark: has("by_dark"), aims: number("aims=").map(|ticks| ticks as u32), volley: value("volley=").and_then(|volley| volley.split_once('x')).and_then(|(shots, gap)| Some((shots.parse().ok()?, gap.parse().ok()?))), shot: value("shot="), calms: number("calms=").map(|ticks| ticks as u32), once: has("once") });
    let roams = value("roams=").and_then(|roams| roams.split_once('x')).and_then(|(across, up)| Some([across.parse().ok()?, up.parse().ok()?]));
    let swims = number("swims=").zip(roams);
    MobTraits { fire_immune: has("fire_immune"), burns_in_sun: has("burns_in_sun"), no_fall: has("no_fall"), breathes_water: has("breathes_water"), breath: number("breath=").map(|ticks| ticks as u32), teleports: has("teleports"), floats: has("floats"), steadfast: has("steadfast"), resists: if has("steadfast") { 1.0 } else { number("resists=").unwrap_or(0.0) }, unpushable: has("unpushable"), experience, hunts, strolls: number("strolls="), panics: number("panics="), swims }
}

/// The mob `id` has `left` of `max` health.
pub fn mob_health_update(id: u64, left: f32, max: f32) -> UpdateAttributes {
    let health = PlayerAttributesItem { min: 0.0, max, current: left, default_min: 0.0, default_max: max, default: max, name: HEALTH.to_owned(), modifiers: Vec::new() };
    UpdateAttributes { runtime_entity_id: id, attributes: vec![health], tick: 0 }
}

/// The flags BDS announced a grown mob of `identifier` with, and `extra` besides.
pub fn grown_flags(identifier: &str, extra: MetadataFlags1) -> MetadataFlags1 {
    MetadataFlags1(mob_flags(identifier).0 | extra.0)
}

/// The mob `id`, whose flags are now `flags` (`IGNITED` among them), has a lit fuse with `left` ticks to go.
pub fn mob_fuse(id: u64, flags: MetadataFlags1, left: i32) -> SetEntityData {
    let metadata = vec![
        MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) },
        MetadataDictionaryItem { key: Key::FuseLength, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(left)) },
    ];
    SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// The mob `id`, whose flags are now `flags` (`EVOKER_SPELL` among them while it casts), casts a
/// spell of `colour` (ARGB), or stops casting.
pub fn mob_spell(id: u64, flags: MetadataFlags1, colour: Option<i32>) -> SetEntityData {
    let mut metadata = vec![MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) }];
    // BDS never clears the colour; it rewrites it with the next spell.
    if let Some(colour) = colour {
        metadata.push(MetadataDictionaryItem { key: Key::EvokerSpellCastingColor, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(colour)) });
    }
    SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// The mob `id`'s flags are now `flags`.
pub fn mob_flags_set(id: u64, flags: MetadataFlags1) -> SetEntityData {
    let item = MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) };
    SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// The mob `id`, whose flags are now `flags` (`CHARGED` among them while it does), readies a shot
/// (a ghast's open mouth) or stops.
pub fn mob_charge(id: u64, flags: MetadataFlags1, charged: bool) -> SetEntityData {
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

/// The effects of the ids `effects` shown on the entity `id`, as BDS packs `VisibleMobEffects` for a
/// mob and a player alike (research/captures/bds-mob-effects.txt, bds-effects.txt): each id doubled,
/// 7 bits apiece, the lowest highest.
pub fn visible_effects(id: u64, effects: impl IntoIterator<Item = i32>) -> SetEntityData {
    let mut ids: Vec<i64> = effects.into_iter().map(i64::from).collect();
    ids.sort_unstable();
    let packed = ids.into_iter().fold(0i64, |packed, effect| (packed << 7) | (effect << 1));
    let item = MetadataDictionaryItem { key: Key::VisibleMobEffects, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(packed)) };
    SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// The mob `id`'s flags, where `BREATHING` came or went, and its breath, where that changed, in
/// one packet as BDS sends them (research/captures/bds-kill.txt); `None` where neither did.
pub fn mob_breath(id: u64, flags: Option<MetadataFlags1>, air: Option<i16>) -> Option<SetEntityData> {
    let flags = flags.map(|flags| MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) });
    let air = air.map(|air| mob_air(id, air).metadata.remove(0));
    let metadata: Vec<_> = flags.into_iter().chain(air).collect();
    (!metadata.is_empty()).then(|| SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_spares_a_strider_a_fall_a_chicken_and_water_a_cod() {
        assert_eq!(mob_traits("minecraft:pig"), MobTraits { experience: Some((1, 3)), strolls: Some(0.25), panics: Some(0.3125), floats: true, ..MobTraits::default() });
        assert_eq!((mob_traits("minecraft:creeper").experience, mob_traits("minecraft:armor_stand").experience), (Some((5, 5)), None));
        assert!(mob_traits("minecraft:strider").fire_immune && mob_traits("minecraft:chicken").no_fall && mob_traits("minecraft:cod").breathes_water);
        assert!(!mob_traits("minecraft:chicken").fire_immune);
        assert_eq!(mob_traits("minecraft:cod").swims, Some((0.1, [16, 4])));
        assert_eq!(["minecraft:hoglin", "minecraft:ravager", "minecraft:warden", "minecraft:cow"].map(|kind| mob_traits(kind).resists), [0.6, 0.75, 1.0, 0.0]);
        assert!(mob_traits("minecraft:shulker").unpushable && mob_traits("minecraft:armor_stand").unpushable && !mob_traits("minecraft:warden").unpushable && !mob_traits("minecraft:cow").unpushable);
    }

    #[test]
    fn a_burning_pig_keeps_its_own_flags() {
        let flags = |on_fire: bool| match mob_flags_set(7, grown_flags("minecraft:pig", MetadataFlags1(i64::from(on_fire)))).metadata[0].value {
            Value::Flags(MetadataFlags1(flags)) => flags,
            _ => unreachable!(),
        };
        assert_eq!(flags(true), flags(false) | 1);
        assert_ne!(flags(false), 0);
        assert_eq!(mob_health_update(7, 3.0, 10.0).attributes[0].current, 3.0);
    }
}
