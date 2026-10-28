//! A creative player's pick block (`BlockPickRequest`), answered as BDS did (research/captures/
//! bds-packets.txt): the block's item is selected where the hotbar holds it, else put in the hand's
//! slot when empty or the first empty one, and the client is sent its windows with the slot
//! selected; viewers see the new hand. An entity pick got no answer and is dropped.
// TODO: the block entity's data a pick may ask for (a chest's items) is left off; blocks whose item
// has another name (wall torches, crops) are not picked.

use acacia_proto::packets::BlockPickRequest;
use bevy_ecs::prelude::*;
use sapling_api::GameMode;
use sapling_world::ItemStack;

use super::{HOTBAR, Inventory, Place, show_hand};
use crate::components::{InLayer, Layer, Mode, Outgoing, PacketQueues};

pub(crate) fn dispatch(world: &mut World) {
    for (player, request) in world.resource_mut::<PacketQueues>().take::<BlockPickRequest>() {
        if world.get::<Mode>(player).is_none_or(|mode| mode.0 != GameMode::Creative) || !crate::blocks::reaches(world, player, [request.x, request.y, request.z]) {
            continue;
        }
        let Some(stack) = picked(world, player, [request.x, request.y, request.z]) else { continue };
        let pos = [request.x, request.y, request.z];
        if !crate::blocks::block_at(world, world.get::<InLayer>(player).map_or(player, |layer| layer.0), pos).is_some_and(|block| crate::hand_events::may_pick(world, player, pos, block)) {
            continue;
        }
        let mut query = world.query::<(&mut Inventory, &mut Outgoing)>();
        let Ok((mut inventory, mut out)) = query.get_mut(world, player) else { continue };
        let holds = |inventory: &Inventory, slot: usize| inventory.slot(Place::Main(slot)).and_then(Option::as_ref).is_some_and(|held| held.stack.name == stack.name && held.stack.tag.is_none());
        let empty = |inventory: &Inventory, slot: usize| inventory.slot(Place::Main(slot)).is_some_and(Option::is_none);
        let selected = usize::from(inventory.selected);
        let slot = if holds(&inventory, selected) {
            selected
        } else if let Some(slot) = (0..HOTBAR).find(|slot| holds(&inventory, *slot)) {
            slot
        } else {
            let slot = if empty(&inventory, selected) { selected } else { (0..HOTBAR).find(|slot| empty(&inventory, *slot)).unwrap_or(selected) };
            inventory.put(Place::Main(slot), Some(stack));
            slot
        };
        inventory.selected = slot as u8;
        inventory.send_all(&mut out, true);
        show_hand(world, player);
    }
}

/// One of the item of the block at `pos`, with the block's state.
fn picked(world: &World, player: Entity, [x, y, z]: [i32; 3]) -> Option<ItemStack> {
    let layer = world.get::<Layer>(world.get::<InLayer>(player)?.0)?;
    let (registry, id) = (layer.world.registry(), layer.world.block(x, y, z));
    let name = registry.get(id).filter(|state| !state.is_air())?.name;
    sapling_data::is_item(name).then(|| ItemStack { name: name.to_owned(), count: 1, damage: 0, block: Some(registry.network_hash(id)), tag: None })
}
