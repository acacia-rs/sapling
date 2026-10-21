//! The fight's count of the crystals on its spikes (`updateCrystalCount` 1482733c0), which the
//! dragon's paths and landing odds go by: every 100 ticks while its dragon lives, and as one blows up.

use bevy_ecs::prelude::*;

use super::{Crystal, Dragon, Fight, crystals, hooks};
use crate::body::Body;
use crate::components::InLayer;

const EVERY: u32 = 100;

fn count(world: &mut World, layer: Entity) {
    let spikes = super::spikes(world, layer);
    let count = world.query::<(&InLayer, &Body, &Crystal)>().iter(world).filter(|(in_layer, body, _)| in_layer.0 == layer && crystals::on_spike(&spikes, body.at)).count() as u32;
    if let Some(mut fight) = world.get_mut::<Fight>(layer) {
        fight.crystals = count;
    }
}

pub(super) fn now_and_then(world: &mut World, layer: Entity) {
    let Some(mut fight) = world.get_mut::<Fight>(layer).filter(|fight| !fight.record.killed) else { return };
    fight.crystal_count += 1;
    if fight.crystal_count >= EVERY {
        fight.crystal_count = 0;
        count(world, layer);
    }
}

/// A crystal of the End `layer` blew up (14829b890): the count again, and the dragon told.
pub(super) fn crystal_gone(world: &mut World, layer: Entity, crystal: Entity, at: [f32; 3]) {
    count(world, layer);
    let dragon = world.get::<Fight>(layer).and_then(|fight| fight.dragon);
    if let Some(dragon) = dragon.filter(|dragon| world.get::<Dragon>(*dragon).is_some()) {
        hooks::crystal_destroyed(world, layer, dragon, crystal, at);
    }
}
