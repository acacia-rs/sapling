//! A world's game rules (`sapling_world::Level::rules`): read by handlers, and set with the
//! world's players told, as BDS tells them, of the one rule that changed. sapling's own rules
//! (`sapling:hunger`, ..) switch what no BDS rule does and are never told.

use bevy_ecs::prelude::*;
use sapling_api::RuleValue;
use sapling_data::{DAYLIGHT_CYCLE, GameRules};

use crate::components::Layer;
use crate::creatures::Cause;
use crate::{chat, clock, dimensions};

const FALL_DAMAGE: &str = "fallDamage";
const FIRE_DAMAGE: &str = "fireDamage";
const DROWNING_DAMAGE: &str = "drowningDamage";
const FREEZE_DAMAGE: &str = "freezeDamage";

fn to_api(value: sapling_data::RuleValue) -> RuleValue {
    match value {
        sapling_data::RuleValue::Bool(on) => RuleValue::Bool(on),
        sapling_data::RuleValue::Int(n) => RuleValue::Int(n),
    }
}

fn from_api(value: RuleValue) -> sapling_data::RuleValue {
    match value {
        RuleValue::Bool(on) => sapling_data::RuleValue::Bool(on),
        RuleValue::Int(n) => sapling_data::RuleValue::Int(n),
    }
}

pub(crate) fn all(world: &World, layer: Entity) -> Vec<(String, RuleValue)> {
    let Some(layer) = world.get::<Layer>(layer) else { return Vec::new() };
    layer.world.level().rules.iter().map(|(name, value)| (name.to_owned(), to_api(value))).collect()
}

/// Whether the switch `name` of the world `layer` is on; false for a number or no such world.
pub(crate) fn on(world: &World, layer: Entity, name: &str) -> bool {
    world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(name))
}

/// The number `name` of the world `layer`; `None` for a switch or no such world.
pub(crate) fn number(world: &World, layer: Entity, name: &str) -> Option<i32> {
    match world.get::<Layer>(layer)?.world.level().rules.get(name)?.1 {
        sapling_data::RuleValue::Int(n) => Some(n),
        sapling_data::RuleValue::Bool(_) => None,
    }
}

/// Whether `rules` spare a player a hit of `cause`: BDS's fall, fire, drowning and freezing
/// switches are for players alone.
pub(crate) fn spares_player(rules: &GameRules, cause: Cause) -> bool {
    let rule = match cause {
        Cause::Fall => FALL_DAMAGE,
        Cause::Fire | Cause::FireTick | Cause::Lava | Cause::Magma | Cause::Campfire | Cause::SoulCampfire => FIRE_DAMAGE,
        Cause::Drowning => DROWNING_DAMAGE,
        Cause::Freezing => FREEZE_DAMAGE,
        _ => return false,
    };
    !rules.on(rule)
}

/// Sets the rule `name` of the world `layer` (of its overworld, for another dimension); false
/// for no such rule or a value of the other kind.
pub(crate) fn set(world: &mut World, layer: Entity, name: &str, value: RuleValue) -> bool {
    let home = dimensions::home(world, layer);
    let Some(mut level) = world.get_mut::<Layer>(home) else { return false };
    let rules = &mut level.world.level_mut().rules;
    let Some(changed) = rules.set(name, from_api(value)) else { return false };
    let told = rules.changed(name).filter(|_| changed);
    let starts_or_stops = rules.get(name).is_some_and(|(own, _)| own == DAYLIGHT_CYCLE);
    dimensions::share_levels(world);
    let Some(told) = told else { return true };
    let told = sapling_net::encode(&told);
    dimensions::members(world, layer).into_iter().for_each(|member| chat::send_to_layer(world, member, &told));
    if starts_or_stops {
        clock::tell_layer(world, layer);
    }
    true
}
