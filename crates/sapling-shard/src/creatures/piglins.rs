//! Piglins' bartering, by the pack: an adult a player clicks holding a gold ingot (`minecraft:interact`
//! with `barter` and `admire`) takes one and admires it in its offhand for 8 seconds
//! (`admire_item`), then hands back a roll of `loot_tables/entities/piglin_barter.json`.
//! research/captures/bds-barter.txt: the ingot shown in the offhand (`MobEquipment` window
//! `Offhand`, slot 1) the tick it was taken, `admire` from its head a tick later, the hand emptied
//! 161 ticks on. Gold lying near it it picks up (pickup.rs).
// TODO: BDS's handing back was not seen (its piglin in the drill picked a thrown ingot up and kept
// it); here the roll drops at the piglin as the hand empties. One that was hit still barters
// (`cooldown_after_being_attacked` 20).

mod pickup;

pub(super) use pickup::steer;

use acacia_proto::packets::MobEquipment;
use acacia_proto::types::WindowID;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{GameMode, Item};

use super::Dying;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Mode, NetId, Tick};
use crate::entities::{self, Kind};
use crate::inventory::{self, Inventory};
use crate::{drops, late_sounds};

const PIGLIN: &str = "minecraft:piglin";
const GOLD: &str = "minecraft:gold_ingot";
const BARTER: &str = "loot_tables/entities/piglin_barter.json";
const ADMIRES: u64 = 161;
/// `admire` is heard from this much of its height up.
const HEAD: f32 = 0.85;
const OFFHAND: u8 = 1;
const LIFT: f32 = 0.5;

/// The tick a piglin's admiring ends, and whether it then barters what it admired.
#[derive(Component)]
pub(super) struct Admiring {
    until: u64,
    barters: bool,
}

/// The gold nuggets a piglin picked up, shown in its main hand.
#[derive(Component)]
pub(super) struct Nuggets(u8);

fn offhand(id: u64, item: Option<&str>) -> MobEquipment {
    let stack = item.map(|name| sapling_data::Stack { name, count: 1, damage: 0, block: None, tag: None });
    let item = stack.and_then(|stack| sapling_data::mob_hand(id, &stack, true)).map_or_else(sapling_data::no_item, |held| held.item);
    MobEquipment { runtime_entity_id: id, item, slot: OFFHAND, selected_slot: 0, window_id: WindowID::Offhand }
}

/// What shows the nuggets a piglin picked up in its main hand.
pub(super) fn held(world: &World, piglin: Entity, id: u64) -> Option<Bytes> {
    let count = world.get::<Nuggets>(piglin)?.0;
    let stack = sapling_data::Stack { name: pickup::NUGGET, count, damage: 0, block: None, tag: None };
    sapling_data::mob_hand(id, &stack, false).map(|held| sapling_net::encode(&held))
}

/// The player clicked `piglin`: false unless it is a grown one, not admiring, and the player holds gold.
pub(crate) fn barter(world: &mut World, player: Entity, piglin: Entity) -> bool {
    let is_piglin = world.get::<Kind>(piglin).is_some_and(|kind| kind.0 == PIGLIN) && world.get::<Dying>(piglin).is_none();
    let holds_gold = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.name == GOLD)).unwrap_or(false);
    if !is_piglin || !holds_gold || super::is_baby(world, piglin) || world.get::<Admiring>(piglin).is_some() {
        return false;
    }
    let (Some(id), Some(body), Some(layer)) = (world.get::<NetId>(piglin).map(|id| id.0), world.get::<Body>(piglin).copied(), world.get::<InLayer>(piglin).map(|layer| layer.0)) else { return false };
    if world.get::<Mode>(player).is_none_or(|mode| mode.0 != GameMode::Creative) {
        inventory::use_up_held(world, player);
    }
    let now = world.resource::<Tick>().0;
    world.entity_mut(piglin).insert(Admiring { until: now + ADMIRES, barters: true });
    let cell = body.at.map(|c| c.floor() as i32);
    entities::tell_viewers(world, piglin, layer, cell, vec![sapling_net::encode(&offhand(id, Some(GOLD)))]);
    late_sounds::play_later(world, 1, layer, cell, super::hunting::sound("admire", id, PIGLIN, &body, HEAD));
    true
}

/// Piglins done admiring empty their hand and hand back their barter.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let done: Vec<(Entity, Entity, u64, Body, bool)> = world
        .query::<(Entity, &Admiring, &InLayer, &NetId, &Body)>()
        .iter(world)
        .filter(|(_, admiring, ..)| now >= admiring.until)
        .map(|(piglin, admiring, layer, id, body)| (piglin, layer.0, id.0, *body, admiring.barters))
        .collect();
    for (piglin, layer, id, body, barters) in done {
        world.entity_mut(piglin).remove::<Admiring>();
        entities::tell_viewers(world, piglin, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&offhand(id, None))]);
        if !barters {
            continue;
        }
        let loot = {
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            sapling_data::table_loot(BARTER, &mut || dice.fraction())
        };
        let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { continue };
        let stacks: Vec<_> = loot.into_iter().filter_map(|(name, count)| inventory::stored(&Item::new(name, count.min(64) as u8), registry)).collect();
        for stack in stacks {
            drops::toss(world, layer, stack, [body.at[0], body.at[1] + LIFT, body.at[2]]);
        }
    }
}
