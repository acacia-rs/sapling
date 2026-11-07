//! An allay handed an item (`allay.json` `minecraft:interact`, `give_item` and `take_item`), as BDS's
//! took one (research/captures/bds-ak1allay.txt, bds-r1bundle.txt): a click holding anything but a lead,
//! not sneaking, on an allay with an empty hand puts one of it in the allay's hand (MobEquipment) and
//! takes one from the player's in that tick; the next tick it is heard `item_given` and shown busy
//! (`ACTION`) and owned (`OwnerEid`, the giver). A click holding something on an allay that holds
//! something does nothing; an empty-handed one takes its item back (the pack's `take_item`, not drilled).
// TODO: what it does with the item (collecting more, `go_and_give_items_to_owner`, note blocks), `ambient.tame`.

use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use bevy_ecs::prelude::*;
use sapling_api::MobSlot;
use sapling_world::ItemStack;

use super::{products, shown};
use crate::components::{NetId, Tick};
use crate::entities::Kind;
use crate::inventory::{self, Inventory};
use crate::movement::Motion;

const ALLAY: &str = "minecraft:allay";

/// An allay handed an item, shown so the tick after.
#[derive(Component)]
pub(super) struct Given {
    by: u64,
    due: u64,
}

fn held_by_allay(world: &World, allay: Entity) -> Option<ItemStack> {
    super::super::command_gear::gear_of(world, allay).into_iter().find(|(slot, _)| *slot == MobSlot::MainHand).and_then(|(_, item)| item)
}

/// The player clicked `entity`; true where it was an allay, which takes the click whatever it does.
pub(super) fn hand(world: &mut World, player: Entity, entity: Entity) -> bool {
    if world.get::<Kind>(entity).is_none_or(|kind| kind.0 != ALLAY) || world.get::<Motion>(player).is_some_and(|motion| motion.pose.sneaking) {
        return false;
    }
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.clone()));
    match (held, held_by_allay(world, entity)) {
        (Some(held), _) if held.name == super::leads::LEAD => return false,
        (Some(held), None) => {
            let one = ItemStack { count: 1, ..held };
            if super::super::command_gear::set(world, entity, MobSlot::MainHand, Some(one)).is_ok() {
                products::spend(world, player);
                let (Some(&NetId(by)), now) = (world.get::<NetId>(player), world.resource::<Tick>().0) else { return true };
                world.entity_mut(entity).insert(Given { by, due: now + 1 });
            }
        }
        (None, Some(item)) => {
            if super::super::command_gear::set(world, entity, MobSlot::MainHand, None).is_ok() {
                inventory::give_and_pop(world, player, &item);
                shown::flag(world, entity, Flags::ACTION, false);
            }
        }
        _ => {}
    }
    true
}

/// Shows the allays handed an item last tick.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<(Entity, u64)> = world.query::<(Entity, &Given)>().iter(world).filter(|(_, given)| given.due <= now).map(|(allay, given)| (allay, given.by)).collect();
    for (allay, by) in due {
        world.entity_mut(allay).remove::<Given>();
        shown::sound(world, allay, "item_given");
        let owner = MetadataDictionaryItem { key: Key::OwnerEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(by as i64)) };
        shown::flag_with(world, allay, Flags::ACTION, true, vec![owner]);
    }
}
