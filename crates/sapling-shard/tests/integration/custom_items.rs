//! Custom items a plugin registers: listed in ItemRegistry and the creative inventory, held,
//! stacked to their own size, thrown and picked up, and kept with the player.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, CreativeContent, InventoryContent, ItemRegistry};
use acacia_proto::types::{
    ContainerSlotType as Container, ItemStackRequestActionsItemContent as Action, ItemStackRequestActionsItemContentCraftCreative as Pick,
    ItemStackRequestActionsItemContentDrop as Throw, ItemStackResponsesItemStatus as Status,
};
use common::requests::{answer, request, slot, take};
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_api::{ArmourSlot, CustomFood, CustomItem, GameMode, Item, PlayerId};
use sapling_world::World;

const STEVE: PlayerId = PlayerId(1);

fn register() -> (i16, i16) {
    let gem = sapling_data::register_item(&CustomItem::new("shard_test:gem", "Gem", Vec::new()).max_stack(16)).unwrap();
    let crown = CustomItem::new("shard_test:crown", "Crown", Vec::new()).armour(ArmourSlot::Head, 4).durability(100);
    let crown = sapling_data::register_item(&crown).unwrap();
    sapling_data::register_item(&CustomItem::new("shard_test:bun", "Bun", Vec::new()).food(CustomFood::new(5, 0.6))).unwrap();
    (gem, crown)
}

#[test]
fn a_joining_client_is_told_the_custom_items_and_lists_them_in_creative() {
    let (gem, crown) = register();
    let mut h = Harness::new();
    h.enter(1, "Steve");
    let packets: Vec<RawPacket> = sapling_data::registry_packets().iter().map(|p| RawPacket::parse(p.clone()).unwrap()).collect();
    let registry: ItemRegistry = packets[0].decode().unwrap();
    let entry = registry.itemstates.iter().find(|item| item.name == "shard_test:gem").expect("the gem is registered");
    assert_eq!((entry.runtime_id, entry.component_based), (gem, true));
    let components = entry.nbt.decode().unwrap().value;
    let properties = components.get("components").and_then(|c| c.get("item_properties")).expect("item_properties");
    assert_eq!(properties.get("max_stack_size"), Some(&acacia_proto::nbt::Value::Int(16)));
    assert!(registry.itemstates.iter().any(|item| item.runtime_id == crown));

    let content: CreativeContent = packets[5].decode().unwrap();
    let listed = content.items.iter().find(|entry| entry.item.network_id == i32::from(gem)).expect("the gem in the creative inventory");
    assert_eq!(content.groups[listed.group_index as usize].category.to_raw(), 4, "listed under items");
    h.remote.run(|ctx| ctx.set_game_mode(STEVE, GameMode::Creative));
    h.tick(1);
    h.receive(1, &request(-3, vec![Action::CraftCreative(Pick { item_id: listed.entry_id, times_crafted: 1 }), take(16, slot(Container::CreativeOutput, 50, -3), slot(Container::Hotbar, 0, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
    h.remote.run(|ctx| assert_eq!(ctx.held_item(STEVE).map(|item| (item.name, item.count)), Some(("shard_test:gem".to_owned(), 16))));
    h.tick(1);
}

#[test]
fn custom_items_stack_to_their_own_size_and_are_kept_with_the_player() {
    let (gem, crown) = register();
    let storage = floor_with(Vec::new());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| _ = ctx.give(STEVE, &Item::new("shard_test:gem", 20)));
    h.remote.run(|ctx| _ = ctx.give(STEVE, &Item::new("shard_test:crown", 1)));
    let sent: Vec<RawPacket> = (0..3).flat_map(|_| h.tick(1)).collect();
    let window = all::<InventoryContent>(&sent).into_iter().filter(|window| window.input.len() == 36).last().expect("the inventory").input;
    let held: Vec<(i16, u16)> = window.iter().filter(|item| item.network_id != 0).map(|item| (item.network_id, item.count)).collect();
    assert_eq!(held, [(gem, 16), (gem, 4), (crown, 1)], "sixteen to a slot");
    assert_eq!(sapling_data::armour("shard_test:crown"), Some([4.0, 0.0]));
    assert_eq!(sapling_data::gear("shard_test:crown").map(|gear| gear.lasts), Some(100));
    assert!(sapling_data::food("shard_test:bun").is_some_and(|bun| bun.hunger == 5.0));

    let stack_id = window[1].stack_id.expect("a stack id");
    h.receive(1, &request(-3, vec![Action::Drop(Throw { count: 4, source: slot(Container::Hotbar, 1, stack_id), randomly: false })]));
    let thrown: Vec<RawPacket> = (0..3).flat_map(|_| h.tick(1)).collect();
    assert!(all::<AddItemEntity>(&thrown).iter().any(|item| item.item.network_id == gem && item.item.count == 4), "a thrown gem shows as one");

    h.leave(1);
    (0..20).for_each(|_| h.shard.tick());
    enter_stored(&mut h, 2);
    h.remote.run(|ctx| assert_eq!(ctx.held_item(PlayerId(2)).map(|item| (item.name, item.count)), Some(("shard_test:gem".to_owned(), 16)), "kept by name"));
    h.tick(2);
}
