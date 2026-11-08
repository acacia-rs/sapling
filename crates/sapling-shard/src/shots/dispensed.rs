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
/// An arrow's uncertainty, less by this for each level of difficulty: 20, 16 and 12 from easy to hard
/// (bds-r1disp.txt, bds-p3disp.txt).
const ARROW_UNCERTAINTY: [f32; 2] = [24.0, 4.0];

/// The shot `item` makes out of a dispenser, its power and uncertainty (an arrow's by the world's
/// difficulty, [`ARROW_UNCERTAINTY`]); snowballs dead straight (bds-p2shoot.txt). See docs/research/dispensers.md.
fn kind_of(item: &ItemStack) -> Option<(Shot, f32, f32)> {
    let potion = |shot: Shot| Some((shot, 0.5, 0.0));
    match item.name.as_str() {
        name if name == ARROW.kind || name == "minecraft:spectral_arrow" => Some((PLAYER_ARROW, 1.1, 0.0)),
        "minecraft:snowball" => Some((SNOWBALL, 1.5, 0.0)),
        "minecraft:egg" | "minecraft:blue_egg" | "minecraft:brown_egg" => Some((EGG, 1.5, 0.0)),
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
    let width = if shot.kind == PLAYER_ARROW.kind { crate::creatures::spread(world, ARROW_UNCERTAINTY) } else { SPREAD * uncertainty };
    let motion = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        if shot.kind == FIREBALL.kind {
            way.map(|c| (c + dice.gaussian() * CHARGE_SPREAD) * power)
        } else {
            let aimed = [way[0], way[1] + RAISED, way[2]];
            let long = aimed.iter().map(|c| c * c).sum::<f32>().sqrt();
            aimed.map(|c| (c / long + dice.gaussian() * width) * power)
        }
    };
    fire(world, layer, from, motion, shot, item)
}

/// Shoots `item` from `from` along the unit way `way` with no shooter, as BDS's
/// `Spawner::spawnProjectile` (14118d160) does for an ominous item spawner: at the power and
/// `uncertainty_base` of the shot's entity JSON, unraised (bds-u1omen.txt). False for what it does not shoot.
pub(crate) fn shoot_unowned(world: &mut World, layer: Entity, from: [f32; 3], way: [f32; 3], item: &ItemStack) -> bool {
    let Some((shot, ..)) = kind_of(item) else { return false };
    let (power, uncertainty) = match item.name.as_str() {
        "minecraft:fire_charge" => (1.3, 10.0),
        "minecraft:wind_charge" => (1.5, 1.0),
        name if name == ARROW.kind => (1.1, 16.0),
        _ => (0.5, 0.0),
    };
    let motion = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        way.map(|c| (c + dice.gaussian() * SPREAD * uncertainty) * power)
    };
    fire(world, layer, from, motion, shot, item)
}

fn fire(world: &mut World, layer: Entity, from: [f32; 3], motion: [f32; 3], shot: Shot, item: &ItemStack) -> bool {
    let loosed = (shot.kind == ARROW.kind).then(|| Loosed { player: None, tipped: ammo::tip(item), ..Loosed::new(Entity::PLACEHOLDER, Pickup::Anyone, Some(ItemStack { count: 1, ..item.clone() })) });
    let Some(fired) = crate::creatures::launch(world, Launch { layer, by: Entity::PLACEHOLDER, archer: ARCHER.into(), from, motion, shot, gives: None }, loosed) else { return false };
    crate::creatures::first_move(world, fired);
    true
}
