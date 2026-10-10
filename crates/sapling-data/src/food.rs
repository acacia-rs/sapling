//! What eating an item gives: hunger and saturation as the Java game and Dragonfly have them
//! (BDS's apple drilled the same, research/captures/bds-eat.txt).
// TODO: stews leave no bowl and what a food does besides feeding (poison, hunger, a golden
// apple's effects) is not here; drinks are not food.

/// Ticks use is held on a food before it is eaten.
const USE_TICKS: u32 = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Food {
    pub hunger: f32,
    pub saturation: f32,
    /// Eaten with a full hunger too.
    pub always: bool,
    pub ticks: u32,
}

const fn plain(hunger: f32, saturation: f32) -> Food {
    Food { hunger, saturation, always: false, ticks: USE_TICKS }
}

/// What the item `name` (`minecraft:bread`) feeds; `None` for what is not eaten.
pub fn food(name: &str) -> Option<Food> {
    Some(match name.strip_prefix("minecraft:").unwrap_or(name) {
        "apple" | "chorus_fruit" => plain(4.0, 2.4),
        "baked_potato" | "bread" | "cooked_cod" | "cooked_rabbit" => plain(5.0, 6.0),
        "beef" | "porkchop" | "rabbit" => plain(3.0, 1.8),
        "cooked_beef" | "cooked_porkchop" => plain(8.0, 12.8),
        "beetroot" => plain(1.0, 1.2),
        "beetroot_soup" | "mushroom_stew" | "cooked_chicken" | "suspicious_stew" => plain(6.0, 7.2),
        "carrot" => plain(3.0, 3.6),
        "cod" | "salmon" | "cookie" | "sweet_berries" | "glow_berries" => plain(2.0, 0.4),
        "cooked_mutton" | "cooked_salmon" => plain(6.0, 9.6),
        "dried_kelp" => Food { ticks: USE_TICKS / 2, ..plain(1.0, 0.2) },
        "tropical_fish" | "pufferfish" => plain(1.0, 0.2),
        "golden_carrot" => plain(6.0, 14.4),
        "melon_slice" | "mutton" | "chicken" | "poisonous_potato" => plain(2.0, 1.2),
        "potato" => plain(1.0, 0.6),
        "pumpkin_pie" => plain(8.0, 4.8),
        "rabbit_stew" => plain(10.0, 12.0),
        "golden_apple" | "enchanted_golden_apple" => Food { always: true, ..plain(4.0, 9.6) },
        "rotten_flesh" => plain(4.0, 0.8),
        "spider_eye" => plain(2.0, 3.2),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_apple_feeds_as_bds_fed_one_and_a_stone_nothing() {
        assert_eq!(food("minecraft:apple"), Some(Food { hunger: 4.0, saturation: 2.4, always: false, ticks: 32 }));
        assert_eq!((food("minecraft:stone"), food("dried_kelp").map(|food| food.ticks)), (None, Some(16)));
        assert!(food("minecraft:golden_apple").is_some_and(|food| food.always));
    }
}
