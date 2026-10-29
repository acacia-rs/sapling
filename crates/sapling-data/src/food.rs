//! What eating or drinking an item gives: hunger and saturation as the Java game and Dragonfly
//! have them (BDS's apple drilled the same, research/captures/bds-eat.txt), and the effects BDS
//! gave (docs/research/food-effects.md).
// TODO: suspicious stews over data 14 were not drilled and give nothing, as 13 and 14 did on BDS.

/// Ticks use is held on a food before it is eaten.
const USE_TICKS: u32 = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Food {
    pub hunger: f32,
    pub saturation: f32,
    /// Eaten with a full hunger too.
    pub always: bool,
    pub ticks: u32,
    pub gives: &'static [Gives],
    /// The item left in its place.
    pub leaves: Option<&'static str>,
    /// Milk: it ends every effect.
    pub cures: bool,
    /// Honey: it ends poison.
    pub ends_poison: bool,
    /// Drunk, not eaten.
    pub drunk: bool,
    /// An ominous bottle: its omen is given without particles, and it is heard as BDS's
    /// (research/captures/bds-fx3.txt).
    pub ominous: bool,
}

/// An effect a food gives: its name (`sapling_api::Effect::name`), for how many ticks, at what
/// amplifier, and how often of all the times it is eaten.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gives {
    pub effect: &'static str,
    pub ticks: u32,
    pub amplifier: u8,
    pub chance: f32,
}

pub(crate) const fn plain(hunger: f32, saturation: f32) -> Food {
    Food { hunger, saturation, always: false, ticks: USE_TICKS, gives: &[], leaves: None, cures: false, ends_poison: false, drunk: false, ominous: false }
}

/// What the item `name` (`minecraft:bread`) feeds; `None` for what is not eaten.
pub fn food(name: &str) -> Option<Food> {
    if let Some(custom) = crate::custom_items::by_name(name) {
        return custom.food;
    }
    Some(match name.strip_prefix("minecraft:").unwrap_or(name) {
        "apple" | "chorus_fruit" => plain(4.0, 2.4),
        "baked_potato" | "bread" | "cooked_cod" | "cooked_rabbit" => plain(5.0, 6.0),
        // BDS's f32 product, 1.8000001 (bds-foods2.txt).
        "beef" | "porkchop" | "rabbit" => plain(3.0, 3.0 * 0.3 * 2.0),
        "cooked_beef" | "cooked_porkchop" => plain(8.0, 12.8),
        "beetroot" => plain(1.0, 1.2),
        "cooked_chicken" => plain(6.0, 7.2),
        "beetroot_soup" | "mushroom_stew" => Food { leaves: Some("minecraft:bowl"), ..plain(6.0, 7.2) },
        "carrot" => plain(3.0, 3.6),
        "cod" | "salmon" | "cookie" => plain(2.0, 0.4),
        // research/captures/bds-foods.txt and bds-foods2.txt: Bedrock's berries feed as a melon slice.
        "sweet_berries" | "glow_berries" => plain(2.0, 1.2),
        "cooked_mutton" | "cooked_salmon" => plain(6.0, 9.6),
        "dried_kelp" => Food { ticks: USE_TICKS / 2, ..plain(1.0, 0.2) },
        "tropical_fish" => plain(1.0, 0.2),
        "pufferfish" => Food {
            gives: &[Gives { effect: "poison", ticks: 1200, amplifier: 1, chance: 1.0 }, Gives { effect: "nausea", ticks: 300, amplifier: 0, chance: 1.0 }, Gives { effect: "hunger", ticks: 300, amplifier: 2, chance: 1.0 }],
            ..plain(1.0, 0.2)
        },
        "honey_bottle" => Food { always: true, ticks: 40, leaves: Some("minecraft:glass_bottle"), ends_poison: true, ..plain(6.0, 1.2) },
        "golden_carrot" => plain(6.0, 14.4),
        "melon_slice" | "mutton" => plain(2.0, 1.2),
        // 4 times of 10 on BDS.
        "chicken" => Food { gives: &[Gives { effect: "hunger", ticks: 600, amplifier: 0, chance: 0.3 }], ..plain(2.0, 1.2) },
        "poisonous_potato" => Food { gives: &[Gives { effect: "poison", ticks: 100, amplifier: 0, chance: 0.6 }], ..plain(2.0, 1.2) },
        "potato" => plain(1.0, 0.6),
        "pumpkin_pie" => plain(8.0, 4.8),
        "rabbit_stew" => Food { leaves: Some("minecraft:bowl"), ..plain(10.0, 12.0) },
        "golden_apple" => Food { always: true, gives: &[Gives { effect: "regeneration", ticks: 100, amplifier: 1, chance: 1.0 }, Gives { effect: "absorption", ticks: 2400, amplifier: 0, chance: 1.0 }], ..plain(4.0, 9.6) },
        "enchanted_golden_apple" => Food {
            always: true,
            gives: &[
                Gives { effect: "regeneration", ticks: 600, amplifier: 1, chance: 1.0 },
                Gives { effect: "absorption", ticks: 2400, amplifier: 3, chance: 1.0 },
                Gives { effect: "resistance", ticks: 6000, amplifier: 0, chance: 1.0 },
                Gives { effect: "fire_resistance", ticks: 6000, amplifier: 0, chance: 1.0 },
            ],
            ..plain(4.0, 9.6)
        },
        "rotten_flesh" => Food { gives: &[Gives { effect: "hunger", ticks: 600, amplifier: 0, chance: 0.8 }], ..plain(4.0, 0.8) },
        "spider_eye" => Food { gives: &[Gives { effect: "poison", ticks: 100, amplifier: 0, chance: 1.0 }], ..plain(2.0, 3.2) },
        "milk_bucket" => Food { always: true, leaves: Some("minecraft:bucket"), cures: true, drunk: true, ..plain(0.0, 0.0) },
        _ => return None,
    })
}

macro_rules! gives {
    ($($effect:literal $ticks:literal $amplifier:literal),+) => { &[$(Gives { effect: $effect, ticks: $ticks, amplifier: $amplifier, chance: 1.0 }),+] };
}

/// What a potion of the data value `data` gives (research/captures/bds-drink.txt and
/// bds-drink2.txt): nothing for water, the plain ones and the four of effects the shard lacks
/// (43 to 46).
pub fn potion_effects(data: i16) -> &'static [Gives] {
    match data {
        5 => gives!["night_vision" 3600 0],
        6 => gives!["night_vision" 9600 0],
        7 => gives!["invisibility" 3600 0],
        8 => gives!["invisibility" 9600 0],
        9 => gives!["jump_boost" 3600 0],
        10 => gives!["jump_boost" 9600 0],
        11 => gives!["jump_boost" 1800 1],
        12 => gives!["fire_resistance" 3600 0],
        13 => gives!["fire_resistance" 9600 0],
        14 => gives!["speed" 3600 0],
        15 => gives!["speed" 9600 0],
        16 => gives!["speed" 1800 1],
        17 => gives!["slowness" 1800 0],
        18 => gives!["slowness" 4800 0],
        19 => gives!["water_breathing" 3600 0],
        20 => gives!["water_breathing" 9600 0],
        21 => gives!["instant_health" 1 0],
        22 => gives!["instant_health" 1 1],
        23 => gives!["instant_damage" 1 0],
        24 => gives!["instant_damage" 1 1],
        25 => gives!["poison" 900 0],
        26 => gives!["poison" 2400 0],
        27 => gives!["poison" 450 1],
        28 => gives!["regeneration" 900 0],
        29 => gives!["regeneration" 2400 0],
        30 => gives!["regeneration" 450 1],
        31 => gives!["strength" 3600 0],
        32 => gives!["strength" 9600 0],
        33 => gives!["strength" 1800 1],
        34 => gives!["weakness" 1800 0],
        35 => gives!["weakness" 4800 0],
        36 => gives!["wither" 800 1],
        37 => gives!["slowness" 400 3, "resistance" 400 2],
        38 => gives!["slowness" 800 3, "resistance" 800 2],
        39 => gives!["slowness" 400 5, "resistance" 400 3],
        40 => gives!["slow_falling" 1800 0],
        41 => gives!["slow_falling" 4800 0],
        42 => gives!["slowness" 400 3],
        // research/captures/bds-drink2.txt: effect ids 32 to 35, three minutes each.
        43 => gives!["wind_charged" 3600 0],
        44 => gives!["weaving" 3600 0],
        45 => gives!["oozing" 3600 0],
        46 => gives!["infested" 3600 0],
        _ => &[],
    }
}

fn potion(data: i16) -> Food {
    Food { always: true, gives: potion_effects(data), leaves: Some("minecraft:glass_bottle"), drunk: true, ..plain(0.0, 0.0) }
}

/// An ominous bottle's bad omen, its data value the amplifier, for 120000 ticks without particles
/// (research/captures/bds-fx3.txt).
fn ominous_bottle(data: i16) -> Food {
    let gives: &'static [Gives] = match data {
        1 => gives!["bad_omen" 120000 1],
        2 => gives!["bad_omen" 120000 2],
        3 => gives!["bad_omen" 120000 3],
        4 => gives!["bad_omen" 120000 4],
        _ => gives!["bad_omen" 120000 0],
    };
    Food { always: true, gives, drunk: true, ominous: true, ..plain(0.0, 0.0) }
}

/// A suspicious stew of the data value `data`, its flower (research/captures/bds-foods2.txt, bds-foods3.txt).
fn suspicious_stew(data: i16) -> Food {
    let gives: &'static [Gives] = match data {
        0 | 10 => gives!["night_vision" 100 0],
        1 => gives!["jump_boost" 100 0],
        2 => gives!["weakness" 140 0],
        3 | 11 => gives!["blindness" 140 0],
        4 => gives!["poison" 220 0],
        5 | 6 => gives!["saturation" 6 0],
        7 => gives!["fire_resistance" 60 0],
        8 => gives!["regeneration" 140 0],
        9 => gives!["wither" 140 0],
        12 => gives!["nausea" 140 0],
        _ => &[],
    };
    Food { leaves: Some("minecraft:bowl"), gives, ..plain(6.0, 7.2) }
}

/// What the item `name` with the data value `damage` gives one who eats or drinks it.
pub fn consumable(name: &str, damage: i16) -> Option<Food> {
    match name.strip_prefix("minecraft:").unwrap_or(name) {
        "potion" => Some(potion(damage)),
        "ominous_bottle" => Some(ominous_bottle(damage)),
        "suspicious_stew" => Some(suspicious_stew(damage)),
        _ => food(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_apple_feeds_as_bds_fed_one_and_a_stone_nothing() {
        assert_eq!(food("minecraft:apple"), Some(plain(4.0, 2.4)));
        assert_eq!(food("spider_eye").map(|food| food.gives), Some(&[Gives { effect: "poison", ticks: 100, amplifier: 0, chance: 1.0 }][..]));
        assert!(food("milk_bucket").is_some_and(|milk| milk.cures && milk.leaves == Some("minecraft:bucket")));
        let strong_poison = consumable("minecraft:potion", 27).unwrap();
        assert_eq!((strong_poison.gives, strong_poison.drunk, strong_poison.leaves), (&[Gives { effect: "poison", ticks: 450, amplifier: 1, chance: 1.0 }][..], true, Some("minecraft:glass_bottle")));
        let turtle = consumable("minecraft:potion", 39).unwrap().gives;
        assert_eq!((turtle.len(), turtle[0].effect, turtle[0].amplifier, turtle[1].effect, turtle[1].amplifier), (2, "slowness", 5, "resistance", 3));
        assert!((5..=46).all(|data| !consumable("minecraft:potion", data).unwrap().gives.is_empty()) && consumable("minecraft:potion", 47).unwrap().gives.is_empty());
        assert!(consumable("minecraft:potion", 0).unwrap().gives.is_empty() && consumable("minecraft:stone", 0).is_none());
        assert_eq!((food("minecraft:stone"), food("dried_kelp").map(|food| food.ticks)), (None, Some(16)));
        assert!(food("minecraft:golden_apple").is_some_and(|food| food.always));
    }

    #[test]
    fn berries_stews_and_omens_are_as_bds_gave_them() {
        assert_eq!(food("minecraft:glow_berries").map(|food| (food.hunger, food.saturation)), Some((2.0, 1.2)));
        assert_eq!(food("minecraft:beef").map(|food| food.saturation.to_string()).as_deref(), Some("1.8000001"));
        let stew = consumable("minecraft:suspicious_stew", 4).unwrap();
        assert_eq!((stew.gives, stew.leaves), (&[Gives { effect: "poison", ticks: 220, amplifier: 0, chance: 1.0 }][..], Some("minecraft:bowl")));
        let given = |data: i16| consumable("minecraft:suspicious_stew", data).unwrap().gives.iter().map(|gives| (gives.effect, gives.ticks)).collect::<Vec<_>>();
        assert_eq!((given(1), given(10), given(12), given(13)), (vec![("jump_boost", 100)], vec![("night_vision", 100)], vec![("nausea", 140)], vec![]), "bds-foods3.txt");
        let bottle = consumable("minecraft:ominous_bottle", 2).unwrap();
        assert!(bottle.ominous && bottle.gives[0].ticks == 120000 && bottle.gives[0].amplifier == 2 && bottle.leaves.is_none());
    }
}
