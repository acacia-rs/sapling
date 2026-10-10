//! What eating or drinking an item gives: hunger and saturation as the Java game and Dragonfly
//! have them (BDS's apple drilled the same, research/captures/bds-eat.txt), and the effects BDS
//! gave (docs/research/food-effects.md).
// TODO: a suspicious stew and chorus fruit do nothing but feed; a potion of an effect the shard lacks is water; a stew's bowl and a potion's
// bottle are the Java game's, not drilled.

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

const fn plain(hunger: f32, saturation: f32) -> Food {
    Food { hunger, saturation, always: false, ticks: USE_TICKS, gives: &[], leaves: None, cures: false, ends_poison: false, drunk: false }
}

/// What the item `name` (`minecraft:bread`) feeds; `None` for what is not eaten.
pub fn food(name: &str) -> Option<Food> {
    Some(match name.strip_prefix("minecraft:").unwrap_or(name) {
        "apple" | "chorus_fruit" => plain(4.0, 2.4),
        "baked_potato" | "bread" | "cooked_cod" | "cooked_rabbit" => plain(5.0, 6.0),
        "beef" | "porkchop" | "rabbit" => plain(3.0, 1.8),
        "cooked_beef" | "cooked_porkchop" => plain(8.0, 12.8),
        "beetroot" => plain(1.0, 1.2),
        "cooked_chicken" => plain(6.0, 7.2),
        "beetroot_soup" | "mushroom_stew" | "suspicious_stew" => Food { leaves: Some("minecraft:bowl"), ..plain(6.0, 7.2) },
        "carrot" => plain(3.0, 3.6),
        "cod" | "salmon" | "cookie" | "sweet_berries" | "glow_berries" => plain(2.0, 0.4),
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

/// What a potion of the data value `data` gives (research/captures/bds-drink.txt): nothing for
/// water, the plain ones and those of effects the shard lacks.
fn potion(data: i16) -> Food {
    let gives: &'static [Gives] = match data {
        14 => &[Gives { effect: "speed", ticks: 3600, amplifier: 0, chance: 1.0 }],
        15 => &[Gives { effect: "speed", ticks: 9600, amplifier: 0, chance: 1.0 }],
        16 => &[Gives { effect: "speed", ticks: 1800, amplifier: 1, chance: 1.0 }],
        21 => &[Gives { effect: "instant_health", ticks: 1, amplifier: 0, chance: 1.0 }],
        22 => &[Gives { effect: "instant_health", ticks: 1, amplifier: 1, chance: 1.0 }],
        23 => &[Gives { effect: "instant_damage", ticks: 1, amplifier: 0, chance: 1.0 }],
        24 => &[Gives { effect: "instant_damage", ticks: 1, amplifier: 1, chance: 1.0 }],
        25 => &[Gives { effect: "poison", ticks: 900, amplifier: 0, chance: 1.0 }],
        26 => &[Gives { effect: "poison", ticks: 2400, amplifier: 0, chance: 1.0 }],
        27 => &[Gives { effect: "poison", ticks: 450, amplifier: 1, chance: 1.0 }],
        28 => &[Gives { effect: "regeneration", ticks: 900, amplifier: 0, chance: 1.0 }],
        29 => &[Gives { effect: "regeneration", ticks: 2400, amplifier: 0, chance: 1.0 }],
        30 => &[Gives { effect: "regeneration", ticks: 450, amplifier: 1, chance: 1.0 }],
        36 => &[Gives { effect: "wither", ticks: 800, amplifier: 1, chance: 1.0 }],
        _ => &[],
    };
    Food { always: true, gives, leaves: Some("minecraft:glass_bottle"), drunk: true, ..plain(0.0, 0.0) }
}

/// What the item `name` with the data value `damage` gives one who eats or drinks it.
pub fn consumable(name: &str, damage: i16) -> Option<Food> {
    match name.strip_prefix("minecraft:").unwrap_or(name) {
        "potion" => Some(potion(damage)),
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
        assert!(consumable("minecraft:potion", 0).unwrap().gives.is_empty() && consumable("minecraft:stone", 0).is_none());
        assert_eq!((food("minecraft:stone"), food("dried_kelp").map(|food| food.ticks)), (None, Some(16)));
        assert!(food("minecraft:golden_apple").is_some_and(|food| food.always));
    }
}
