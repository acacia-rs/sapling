//! A world's game rules (`sapling_world::Level::rules`): read by handlers, and set with the
//! world's players told, as BDS tells them, of the one rule that changed.

use bevy_ecs::prelude::*;
use sapling_api::RuleValue;
use sapling_data::DAYLIGHT_CYCLE;

use crate::components::Layer;
use crate::{chat, clock};

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
    let Some(layer) = world.get::<Layer>(layer) else {
        return Vec::new();
    };
    layer
        .world
        .level()
        .rules
        .iter()
        .map(|(name, value)| (name.to_owned(), to_api(value)))
        .collect()
}

/// Sets the rule `name` of the world `layer`; false for no such rule or a value of the other kind.
pub(crate) fn set(world: &mut World, layer: Entity, name: &str, value: RuleValue) -> bool {
    let Some(mut level) = world.get_mut::<Layer>(layer) else {
        return false;
    };
    let rules = &mut level.world.level_mut().rules;
    let Some(changed) = rules.set(name, from_api(value)) else {
        return false;
    };
    let Some(told) = rules.changed(name).filter(|_| changed) else {
        return true;
    };
    let starts_or_stops = rules
        .get(name)
        .is_some_and(|(own, _)| own == DAYLIGHT_CYCLE);
    chat::send_to_layer(world, layer, &sapling_net::encode(&told));
    if starts_or_stops {
        clock::tell_layer(world, layer);
    }
    true
}
