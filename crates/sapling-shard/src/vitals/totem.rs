//! A totem of undying held in either hand keeps a player alive through a hit that would kill it,
//! as BDS's did (research/captures/bds-totem.txt): its effects go, it is given regeneration II,
//! absorption II and fire resistance, it is left with 1 health and the totem is used up. The void
//! and `/kill` (which never asks) pass it by.

use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent};
use acacia_proto::types::{Vec3f, WindowIDVarint};
use bevy_ecs::prelude::*;
use sapling_api::Effect;
use sapling_data::EYE_HEIGHT;

use super::{Vitals, effects, tell};
use crate::components::{NetId, Outgoing, Position};
use crate::creatures::Cause;
use crate::inventory::{Inventory, Place};

const TOTEM: &str = "minecraft:totem_of_undying";
/// What it gives: the effect, its ticks and its level from 0.
const GIVES: [(Effect, u32, u8); 3] = [(Effect::Regeneration, 900, 1), (Effect::Absorption, 100, 1), (Effect::FireResistance, 800, 0)];

/// Where the totem is held: the off hand is used first, as BDS used it with one in each.
fn held(inventory: &Inventory) -> Option<Place> {
    let is_totem = |place: Place| inventory.slot(place).and_then(Option::as_ref).is_some_and(|held| held.stack.name == TOTEM);
    [Place::Offhand, Place::Main(usize::from(inventory.selected) % crate::inventory::HOTBAR)].into_iter().find(|place| is_totem(*place))
}

/// Saves a player a hit of `cause` left without health, if it holds a totem; whether it did.
/// Told before the hit's flash, as on BDS.
pub(super) fn rescue(world: &mut World, player: Entity, cause: Cause) -> bool {
    let dead = world.get::<Vitals>(player).is_some_and(|vitals| vitals.health.left() <= 0.0);
    let Some(place) = world.get::<Inventory>(player).and_then(held).filter(|_| dead && cause != Cause::Void) else { return false };
    let (Some(id), Some(feet)) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Position>(player).map(|position| position.feet)) else { return false };
    if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        vitals.health.mend(1.0);
    }
    // BDS took them off in the order of their ids.
    let mut on: Vec<Effect> = Effect::ALL.into_iter().filter(|effect| effects::level(world, player, *effect).is_some()).collect();
    on.sort_by_key(|effect| effect.id());
    for effect in on {
        effects::end(world, player, effect);
    }
    for (effect, ticks, level) in GIVES {
        effects::give(world, player, effect, ticks, level, true);
    }
    tell(world, player, &EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ConsumeTotem, data: 0, fire_at_position: None });
    let [x, y, z] = feet;
    tell(world, player, &LevelEvent { event: LevelEventEvent::SoundTotem, position: Vec3f { x, y: y + EYE_HEIGHT, z }, data: 0 });
    let mut query = world.query::<(&mut Inventory, &mut Outgoing)>();
    if let Ok((mut inventory, mut out)) = query.get_mut(world, player) {
        let _ = inventory.take(place, 1);
        // One in the main hand went without a word to the client.
        if place == Place::Offhand {
            inventory.send_window(&mut out, WindowIDVarint::Offhand);
        }
    }
    true
}
