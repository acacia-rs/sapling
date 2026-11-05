//! The `/mobevent` switches kept with a world (`mobevents`, sapling-leveldb mob_events.rs): taken
//! from the first world added that kept some, handed back to every world as it is saved.

use std::any::TypeId;

use bevy_ecs::prelude::*;
use sapling_api::{ALL_MOB_EVENTS, MOB_EVENTS, MobEvents};

use crate::components::Layer;
use crate::plugins::PluginState;

/// Makes the switches `layer`'s world kept the shard's, unless the shard has its own already.
pub(crate) fn adopt(world: &mut World, layer: Entity) {
    let Some(kept) = world.get::<Layer>(layer).and_then(|layer| layer.world.level().mob_events.clone()) else { return };
    world.init_resource::<PluginState>();
    let mut state = world.resource_mut::<PluginState>();
    if state.0.contains_key(&TypeId::of::<MobEvents>()) {
        return;
    }
    let mut switches = MobEvents::default();
    for (name, on) in kept {
        switches.set(&name, on);
    }
    state.0.insert(TypeId::of::<MobEvents>(), Box::new(switches));
}

/// Hands every world the shard's switches to save, all four as BDS writes them.
pub(crate) fn keep_all(world: &mut World) {
    let switches = crate::plugins::state::<MobEvents>(world);
    let kept: Vec<(String, bool)> = MOB_EVENTS.iter().map(|(name, _)| *name).chain([ALL_MOB_EVENTS]).map(|name| (name.to_owned(), switches.is_none_or(|switches| switches.is_on(name)))).collect();
    for mut layer in world.query::<&mut Layer>().iter_mut(world) {
        layer.world.level_mut().mob_events = Some(kept.clone());
    }
}
