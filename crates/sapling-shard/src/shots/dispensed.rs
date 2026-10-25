//! What a dispenser shoots of the things players loose and throw (docs/research/dispensers.md):
//! arrows, snowballs, eggs, thrown potions, bottles o' enchanting, wind charges and fire charges,
//! out of its face a little upwards, at a power and spread of each kind's.

use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use super::ammo;
use super::kinds::{BOTTLE, EGG, LINGERING, PLAYER_ARROW, SNOWBALL, SPLASH, WIND};
use crate::blocks::Dice;
use crate::creatures::{ARROW, FIREBALL, Lands, Launch, Loosed, Pickup, Shot};

/// A shot's gaussian spread for each point of uncertainty.
const SPREAD: f32 = 0.0075;
/// How much a shot is aimed above the face's way.
const RAISED: f32 = 0.1;
/// A fire charge's gaussian spread about its way.
const CHARGE_SPREAD: f32 = 0.05;
const ARCHER: &str = "dispenser";

/// The shot `item` makes out of a dispenser, its power and uncertainty (the item JSON's; arrows'
/// uncertainty is its base less 4 for each step of normal difficulty, unverified).
fn kind_of(item: &ItemStack) -> Option<(Shot, f32, f32)> {
    let potion = |shot: Shot| Some((shot, 0.5, 6.0));
    match item.name.as_str() {
        name if name == ARROW.kind || name == "minecraft:spectral_arrow" => Some((PLAYER_ARROW, 1.6, 8.0)),
        "minecraft:snowball" => Some((SNOWBALL, 1.5, 6.0)),
        "minecraft:egg" | "minecraft:blue_egg" | "minecraft:brown_egg" => Some((EGG, 1.5, 6.0)),
        "minecraft:splash_potion" => potion(Shot { lands: Lands::Splash(item.damage), ..SPLASH }),
        "minecraft:lingering_potion" => potion(Shot { lands: Lands::Lingers(item.damage), ..LINGERING }),
        "minecraft:experience_bottle" => potion(BOTTLE),
        "minecraft:wind_charge" => Some((WIND, 1.0, 1.0)),
        "minecraft:fire_charge" => Some((FIREBALL, FIREBALL.speed, 0.0)),
        _ => None,
    }
}

/// Shoots `item` from `from` along the unit way `way`; false for what a dispenser does not shoot.
pub(crate) fn shoot(world: &mut World, layer: Entity, from: [f32; 3], way: [f32; 3], item: &ItemStack) -> bool {
    let Some((shot, power, uncertainty)) = kind_of(item) else { return false };
    let motion = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        if shot.kind == FIREBALL.kind {
            way.map(|c| (c + dice.gaussian() * CHARGE_SPREAD) * power)
        } else {
            let aimed = [way[0], way[1] + RAISED, way[2]];
            let long = aimed.iter().map(|c| c * c).sum::<f32>().sqrt();
            aimed.map(|c| (c / long + dice.gaussian() * SPREAD * uncertainty) * power)
        }
    };
    let loosed = (shot.kind == ARROW.kind).then(|| Loosed { player: None, tipped: ammo::tip(item), ..Loosed::new(Entity::PLACEHOLDER, Pickup::Anyone, Some(ItemStack { count: 1, ..item.clone() })) });
    crate::creatures::launch(world, Launch { layer, by: Entity::PLACEHOLDER, archer: ARCHER.into(), from, motion, shot, gives: None }, loosed).is_some()
}
