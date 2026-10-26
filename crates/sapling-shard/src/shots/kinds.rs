//! What players loose and throw, by the vanilla pack's `minecraft:projectile` of each: its
//! `gravity`, its `inertia` (0.99 unless named) and `liquid_inertia` (0.6), its box and its sound.

use sapling_data::shots::TRIDENT_HARM;

use crate::creatures::{ARROW, Lands, Shot};

/// A player's arrow (`player_arrow`): its speed is the draw's.
pub(super) const PLAYER_ARROW: Shot = Shot { spread: 0.0, ..ARROW };
pub(super) const TRIDENT: Shot = Shot {
    kind: "minecraft:thrown_trident",
    weight: 0.1,
    wet_drag: 0.99,
    harm: Some(TRIDENT_HARM),
    sound: "item.trident.throw",
    heard: ["item.trident.hit", "item.trident.hit_ground"],
    size: [0.25, 0.35],
    ..PLAYER_ARROW
};
const THROWN: Shot = Shot { sticks: false, sound: "throw", ..PLAYER_ARROW };
pub(super) const SNOWBALL: Shot = Shot { kind: "minecraft:snowball", weight: 0.03, lands: Lands::Snowball, ..THROWN };
pub(super) const EGG: Shot = Shot { kind: "minecraft:egg", weight: 0.03, lands: Lands::Egg, ..THROWN };
pub(super) const PEARL: Shot = Shot { kind: "minecraft:ender_pearl", weight: 0.025, drag: 1.0, wet_drag: 1.0, lands: Lands::Pearl, ..THROWN };
pub(super) const SPLASH: Shot = Shot { kind: "minecraft:splash_potion", lands: Lands::Splash(0), ..THROWN };
pub(super) const LINGERING: Shot = Shot { kind: "minecraft:lingering_potion", lands: Lands::Lingers(0), ..THROWN };
pub(super) const BOTTLE: Shot = Shot { kind: "minecraft:xp_bottle", lands: Lands::Experience, ..THROWN };
/// What a world folder keeps lying of the kind `kind` (flight.rs `keep`).
pub(super) fn kept(kind: &str) -> Option<Shot> {
    [PLAYER_ARROW, TRIDENT].into_iter().find(|shot| shot.kind == kind)
}

pub(super) const WIND: Shot = Shot { kind: "minecraft:wind_charge_projectile", weight: 0.0, drag: 1.0, wet_drag: 1.0, lands: Lands::Wind, size: [0.3125, 0.3125], ..THROWN };
