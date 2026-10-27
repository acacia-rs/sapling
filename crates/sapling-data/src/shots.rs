//! What players loose and throw, as BDS reckons it (docs/research/projectiles.md): a bow's draw, an
//! arrow's harm (`ImpactDamageSubcomponent::doOnHitEffect` 1424050a0), what bows, crossbows and
//! tridents last, and the vanilla pack's projectile numbers. The enchantments: enchant/archery.rs.

use crate::enchant;

/// A draw is reckoned in seconds of it; less than this much of a full one looses nothing.
pub const LEAST_DRAW: f32 = 0.1;
/// The pack's `power`: what a full draw, a crossbow and a trident loose at, and things thrown.
pub const BOW_SPEED: f32 = 3.0;
pub const CROSSBOW_SPEED: f32 = 3.15;
pub const TRIDENT_SPEED: f32 = 4.0;
pub const THROWN_SPEED: f32 = 1.5;
pub const BOTTLE_SPEED: f32 = 0.5;
/// A bottle's `angle_offset`: it is thrown this many degrees above the look.
pub const BOTTLE_TILT: f32 = -20.0;
/// An arrow costs its speed this many times over (`power_multiplier`).
const POWER_MULTIPLIER: f32 = 2.0;
/// A critical arrow costs this much of its harm more, and this, at most.
const CRITICAL: [f32; 2] = [0.5, 2.0];
/// A tipped arrow gives this much of its potion's time (BDS 142410e20).
pub const TIPPED_SHARE: f32 = 1.0 / 8.0;
/// A trident is thrown after this many ticks of use or more.
pub const TRIDENT_READY: u64 = 10;
pub const TRIDENT_HARM: f32 = 8.0;
/// A snowball costs a blaze this; it and an egg throw what they hit this fast.
pub const SNOWBALL_BLAZE: f32 = 3.0;
pub const KNOCK: f32 = 0.4;
/// An ender pearl costs its thrower this as a fall (BDS 14240a840), may leave an endermite, and
/// cannot be thrown again for this many ticks (142cd0b00).
pub const PEARL_FALL: f32 = 5.0;
pub const ENDERMITE_CHANCE: f32 = 0.05;
pub const PEARL_COOLDOWN: u64 = 20;
/// A wind charge cannot be thrown again for this many ticks.
// TODO: the Java game's; BDS's was not found.
pub const WIND_COOLDOWN: u64 = 10;
/// An egg hatches a chick one time in 8, and four of them one time in 32 of those.
const CHICK: f32 = 0.125;
const FOUR_CHICKS: f32 = 0.031_25;
/// A bottle o' enchanting lets out this much experience and up to this much more, less one.
const BOTTLE: [i32; 2] = [3, 8];
/// The uses a bow, a crossbow and a trident last, Bedrock's.
const LASTS: &[(&str, u16)] = &[("minecraft:bow", 384), ("minecraft:crossbow", 464), ("minecraft:trident", 250)];

/// How much of a full draw `ticks` of drawing a bow are.
pub fn bow_draw(ticks: u64) -> f32 {
    let seconds = ticks as f32 / 20.0;
    ((seconds * seconds + seconds * 2.0) / 3.0).min(1.0)
}

/// What an arrow flying at `speed` costs whom it hits, with Power at `power`; a `critical` one
/// more by `roll` in [0, 1).
pub fn arrow_harm(speed: f32, power: u8, critical: bool, roll: f32) -> f32 {
    let harm = (speed * POWER_MULTIPLIER * enchant::power_bonus(power)).ceil();
    if critical { harm + (harm * CRITICAL[0] + CRITICAL[1]) * roll } else { harm }
}

/// The uses the bow, crossbow or trident `name` lasts.
pub fn shooter_lasts(name: &str) -> Option<u16> {
    LASTS.iter().find(|(known, _)| *known == name).map(|(_, lasts)| *lasts)
}

/// How many chicks an egg hatches, by draws in [0, 1).
pub fn chicks(roll: &mut dyn FnMut() -> f32) -> u32 {
    match roll() < CHICK {
        true if roll() < FOUR_CHICKS => 4,
        true => 1,
        false => 0,
    }
}

/// The experience a bottle o' enchanting lets out, by a draw in [0, 1).
pub fn bottle_experience(roll: &mut dyn FnMut() -> f32) -> i32 {
    BOTTLE[0] + (roll() * BOTTLE[1] as f32) as i32
}

/// Effects' colours: BDS's for the three a witch's splash showed (research/captures/bds-witch.txt)
/// and for speed, which a cauldron showed (bds-cauldrons.txt); the Java game's for the rest.
const COLOURS: &[(&str, u32)] = &[
    ("slowness", 0xFF8B_AFE0),
    ("instant_damage", 0xFFA9_656A),
    ("poison", 0xFF87_A363),
    ("weakness", 0xFF48_4D48),
    ("speed", 0xFF33_EBFF),
    ("strength", 0xFF93_2423),
    ("instant_health", 0xFFF8_2423),
    ("jump_boost", 0xFF22_FF4C),
    ("regeneration", 0xFFCD_5CAB),
    ("resistance", 0xFF99_453A),
    ("fire_resistance", 0xFFE4_9A3A),
    ("water_breathing", 0xFF2E_5299),
    ("invisibility", 0xFF7F_8392),
    ("night_vision", 0xFF1F_1FA1),
    ("wither", 0xFF35_2A27),
    ("slow_falling", 0xFFFF_EFD1),
];
/// Water's, and what a potion of an effect without a colour here shows.
const WATER: u32 = 0xFF38_5DC6;

/// The colour of a thrown potion of the data value `data`: its first effect's.
pub fn potion_colour(data: i16) -> i32 {
    let first = crate::potion_effects(data).first().map(|gives| gives.effect);
    first.and_then(|effect| COLOURS.iter().find(|(known, _)| *known == effect)).map_or(WATER, |(_, colour)| *colour) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draw_and_an_arrows_harm_are_reckoned_as_bds_reckons_them() {
        let drawn = [0, 2, 10, 20, 40].map(bow_draw);
        assert!(drawn.iter().zip([0.0, 0.07, 0.416_666_7, 1.0, 1.0]).all(|(drawn, wanted)| (drawn - wanted).abs() < 1e-6), "{drawn:?}");
        assert_eq!([arrow_harm(1.24, 0, false, 0.9), arrow_harm(2.97, 0, false, 0.0), arrow_harm(2.97, 1, false, 0.0)], [3.0, 6.0, 9.0]);
        assert_eq!([arrow_harm(2.97, 0, true, 0.0), arrow_harm(2.97, 0, true, 0.5)], [6.0, 8.5]);
        assert_eq!([shooter_lasts("minecraft:bow"), shooter_lasts("minecraft:stick")], [Some(384), None]);
    }

    #[test]
    fn eggs_hatch_and_bottles_pour_by_their_draws() {
        let mut draws = [0.1, 0.01].into_iter();
        assert_eq!([chicks(&mut || 0.5), chicks(&mut || 0.1), chicks(&mut || draws.next().unwrap())], [0, 1, 4]);
        assert_eq!([bottle_experience(&mut || 0.0), bottle_experience(&mut || 0.999)], [3, 10]);
        assert_eq!([potion_colour(17), potion_colour(0)], [0xFF8B_AFE0_u32 as i32, 0xFF38_5DC6_u32 as i32]);
    }
}
