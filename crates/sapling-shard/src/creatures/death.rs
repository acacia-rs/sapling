//! A mob's death: what it lets go of, what its viewers are told, and the loot it leaves.

use acacia_proto::packets::MobEquipment;
use acacia_proto::types::WindowID;
use bevy_ecs::prelude::*;
use sapling_api::Item;

use super::health::Health;
use super::{Dying, hunting, omens};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, NetId, Tick};
use crate::{drops, entities, inventory};

const MOB_LOOT: &str = "doMobLoot";
/// How far above its feet a mob's loot appears (where the wire puts it), as BDS was seen to
/// (research/captures/bds-kill.txt).
const LOOT_LIFT: f32 = 0.075;

/// The mob, seen to die, is dead where it is, no longer kept by the world folder, and leaves its
/// loot, cooked if it was on fire. Its viewers see both its hands emptied after its death, as
/// BDS's (research/captures/bds-reach.txt).
pub(super) fn die(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body) {
    if crate::dragon::dies(world, entity) {
        return;
    }
    let now = world.resource::<Tick>().0;
    if let Some(id) = world.get::<NetId>(entity).map(|id| id.0) {
        hunting::let_go(world, entity, layer, id, body);
        let hand = |slot: u8, window_id| sapling_net::encode(&MobEquipment { runtime_entity_id: id, item: sapling_data::no_item(), slot, selected_slot: slot, window_id });
        let emptied = vec![hand(0, WindowID::Inventory), hand(1, WindowID::Offhand)];
        entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), emptied);
    }
    omens::died(world, entity, layer, body);
    world.entity_mut(entity).insert(Dying(now)).remove::<entities::Stored>();
    let on_fire = world.get_mut::<Health>(entity).is_some_and(|mut health| {
        health.end();
        health.is_burning()
    });
    if leaves_loot(world, layer) {
        leave_loot(world, layer, kind, on_fire, body.at);
    }
}

/// Whether the world's `doMobLoot` lets the dead leave loot and experience.
pub(super) fn leaves_loot(world: &World, layer: Entity) -> bool {
    world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(MOB_LOOT))
}

fn leave_loot(world: &mut World, layer: Entity, kind: &str, on_fire: bool, [x, y, z]: [f32; 3]) {
    let loot = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        sapling_data::mob_loot(kind, on_fire, &mut || dice.fraction())
    };
    let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { return };
    let stacks: Vec<_> = loot.into_iter().filter_map(|(name, count)| inventory::stored(&Item::new(name, count.min(64) as u8), registry)).collect();
    for stack in stacks {
        drops::toss(world, layer, stack, [x, y + LOOT_LIFT, z]);
    }
}
