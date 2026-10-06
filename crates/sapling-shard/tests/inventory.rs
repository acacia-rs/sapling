//! A player's items: what the client is sent, what it may ask for, what others see of it, and
//! what becomes of a thrown stack.

mod common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{
    AddItemEntity, AddPlayer, CreativeContent, InventoryContent, MobArmorEquipment, MobEquipment, PlayerHotbar, RemoveEntity,
    ResourcePackClientResponseResponseStatus as PackStatus, TakeItemEntity,
};
use acacia_proto::types::{
    ContainerSlotType as Container, ItemStackRequestActionsItemContent as Action, ItemStackRequestActionsItemContentCraftCreative as Pick,
    ItemStackRequestActionsItemContentDestroy as Destroy, ItemStackRequestActionsItemContentDrop as Throw, ItemStackRequestActionsItemContentPlace as Place,
    ItemStackResponsesItemStatus as Status, WindowID,
};
use common::requests::{answer, request, slot, take};
use common::{Harness, all, hash, input_at, pack_response};
use sapling_api::{DropEvent, GameMode, Handler, Item, Player, PlayerId, ShardCtx, Slot};

const STEVE: PlayerId = PlayerId(1);

#[test]
fn a_joining_player_is_sent_its_windows() {
    let mut h = Harness::new();
    h.join(1, "Steve");
    h.tick(1);
    h.receive(1, &pack_response(PackStatus::HaveAllPacks));
    h.tick(1);
    h.receive(1, &pack_response(PackStatus::Completed));
    let sent = h.tick(1);
    let windows = all::<InventoryContent>(&sent);
    assert_eq!(windows.iter().map(|window| window.input.len()).collect::<Vec<_>>(), [36, 5, 54, 1], "inventory, armour, the screen's slots, offhand");
    assert!(windows.iter().all(|window| window.input.iter().all(|item| item.network_id == 0)));
    let hotbar = &all::<PlayerHotbar>(&sent)[0];
    assert_eq!((hotbar.selected_slot, hotbar.window_id, hotbar.select_slot), (0, WindowID::Inventory, true));
    assert!(all::<InventoryContent>(&h.tick(1)).is_empty(), "once");
}

#[test]
fn requests_move_stacks_and_a_wrong_one_is_undone() {
    let mut h = Harness::new();
    h.enter(1, "Steve");
    h.remote.run(|ctx| {
        assert_eq!(ctx.give(STEVE, &Item::new("minecraft:stone", 70)), 0);
        assert_eq!(ctx.give(STEVE, &Item::new("sapling:no_such_item", 3)), 3);
    });
    let windows = all::<InventoryContent>(&h.tick(1));
    let main = &windows[0].input;
    assert_eq!((windows.len(), main[0].count, main[1].count, main[2].count), (4, 64, 6, 0), "a give fills stacks, then shows every window");
    assert_eq!(main[0].block_runtime_id, hash("minecraft:stone"));
    let (first, second) = (main[0].stack_id.unwrap(), main[1].stack_id.unwrap());
    assert!(first > 0 && second > 0 && first != second);

    // Ten onto the cursor: a part of a stack is a new stack.
    h.receive(1, &request(-3, vec![take(10, slot(Container::Hotbar, 0, first), slot(Container::Cursor, 0, 0))]));
    let (status, slots) = answer(&h.tick(1));
    let cursor = slots[1].3;
    assert_eq!((status, &slots[..]), (Status::Ok, &[(Container::Hotbar, 0, 54, first), (Container::Cursor, 0, 10, cursor)][..]));
    assert!(cursor != first && cursor != second);
    // The cursor onto the six: they become one stack, the six's.
    h.receive(1, &request(-5, vec![Action::Place(Place { count: 10, source: slot(Container::Cursor, 0, cursor), destination: slot(Container::Hotbar, 1, second) })]));
    assert_eq!(answer(&h.tick(1)), (Status::Ok, vec![(Container::Cursor, 0, 0, 0), (Container::Hotbar, 1, 16, second)]));
    // A whole stack keeps its id where it goes.
    h.receive(1, &request(-7, vec![take(54, slot(Container::Hotbar, 0, first), slot(Container::Inventory, 20, 0))]));
    assert_eq!(answer(&h.tick(1)), (Status::Ok, vec![(Container::Hotbar, 0, 0, 0), (Container::Inventory, 20, 54, first)]));

    // More than is there, a stack the client has wrong, a slot that does not exist: nothing
    // happens and the client is shown what it has.
    for wrong in [take(60, slot(Container::Inventory, 20, first), slot(Container::Cursor, 0, 0)), take(1, slot(Container::Inventory, 20, 999), slot(Container::Cursor, 0, 0)), take(1, slot(Container::Inventory, 40, 0), slot(Container::Cursor, 0, 0))] {
        h.receive(1, &request(-9, vec![wrong]));
        let sent = h.tick(1);
        assert_eq!(answer(&sent), (Status::Error, Vec::new()));
        let shown = all::<InventoryContent>(&sent);
        assert_eq!((shown.len(), shown[0].input[20].count, shown[0].input[1].count), (4, 54, 16));
    }
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let items = seen.clone();
    h.remote.run(move |ctx| items.lock().unwrap().extend([Slot::Main(1), Slot::Main(20), Slot::Main(0), Slot::Offhand].map(|slot| ctx.item(STEVE, slot))));
    h.tick(1);
    assert_eq!(*seen.lock().unwrap(), [Some(Item::new("minecraft:stone", 16)), Some(Item::new("minecraft:stone", 54)), None, None]);
}

#[test]
fn the_creative_inventory_gives_items_in_creative_mode_only() {
    let mut h = Harness::new();
    h.enter(1, "Steve");
    let content: CreativeContent = RawPacket::parse(sapling_data::registry_packets()[5].clone()).unwrap().decode().unwrap();
    let stone = content.items.iter().find(|entry| entry.item.block_runtime_id as u32 == hash("minecraft:stone")).expect("stone in the creative inventory").entry_id;
    let pick = |id: i32| request(id, vec![Action::CraftCreative(Pick { item_id: stone, times_crafted: 1 }), take(12, slot(Container::CreativeOutput, 50, id), slot(Container::Hotbar, 3, 0))]);
    h.receive(1, &pick(-3));
    assert_eq!(answer(&h.tick(1)).0, Status::Error, "survival");

    h.remote.run(|ctx| ctx.set_game_mode(STEVE, GameMode::Creative));
    h.tick(1);
    h.receive(1, &pick(-5));
    let (status, slots) = answer(&h.tick(1));
    assert_eq!((status, slots.len(), &slots[0].0, slots[0].1, slots[0].2), (Status::Ok, 1, &Container::Hotbar, 3, 12));
    h.receive(1, &request(-7, vec![Action::Destroy(Destroy { count: 2, source: slot(Container::Hotbar, 3, slots[0].3) })]));
    assert_eq!(answer(&h.tick(1)), (Status::Ok, vec![(Container::Hotbar, 3, 10, slots[0].3)]));
    h.receive(1, &request(-9, vec![Action::CraftCreative(Pick { item_id: u32::MAX, times_crafted: 1 })]));
    assert_eq!(answer(&h.tick(1)).0, Status::Error, "an entry the creative inventory does not have");
}

/// Keeps players from throwing swords away.
struct KeepSwords;

impl Handler for KeepSwords {
    fn on_drop(&mut self, _: &mut ShardCtx<'_>, _: &Player, event: &mut DropEvent) {
        if event.item().name.ends_with("_sword") {
            event.cancel();
        }
    }
}

#[test]
fn others_see_what_a_player_holds_and_wears_and_pick_up_what_it_throws() {
    let mut h = Harness::new();
    h.shard.add_handler(Box::new(KeepSwords));
    h.enter(1, "Steve");
    h.remote.run(|ctx| {
        assert!(ctx.set_item(STEVE, Slot::Main(0), Some(&Item::new("minecraft:diamond_sword", 1))));
        assert!(ctx.set_item(STEVE, Slot::Main(4), Some(&Item::new("minecraft:diamond", 5))));
        assert!(ctx.set_item(STEVE, Slot::Armor(0), Some(&Item::new("minecraft:iron_helmet", 1))));
        assert!(!ctx.set_item(STEVE, Slot::Main(36), None) && !ctx.set_item(STEVE, Slot::Main(1), Some(&Item::new("sapling:no_such_item", 1))));
        assert_eq!(ctx.held_item(STEVE), Some(Item::new("minecraft:diamond_sword", 1)));
    });
    let own = all::<InventoryContent>(&h.tick(1));
    let (sword, diamonds) = (own[own.len() - 4].input[0].clone(), own[own.len() - 4].input[4].clone());

    let sent = h.enter(2, "Alex");
    let steve = &all::<AddPlayer>(sent.to(2))[0];
    assert_eq!((steve.held_item.network_id, steve.held_item.has_stack_id), (sword.network_id, false));
    let worn = &all::<MobArmorEquipment>(sent.to(2))[0];
    assert!(worn.helmet.network_id != 0 && worn.boots.network_id == 0 && worn.runtime_entity_id == steve.runtime_id);

    // Steve switches to the diamonds.
    h.receive(1, &MobEquipment { runtime_entity_id: steve.runtime_id, item: diamonds.clone(), slot: 4, selected_slot: 4, window_id: WindowID::Inventory });
    let sent = h.tick_all();
    let held = &all::<MobEquipment>(sent.to(2))[0];
    assert_eq!((held.item.network_id, held.item.count, held.selected_slot, held.runtime_entity_id), (diamonds.network_id, 5, 4, steve.runtime_id));
    assert!(all::<MobEquipment>(sent.to(1)).is_empty());

    // A handler keeps the sword; the diamonds go and lie in front of Steve for both to see.
    let throw = |from: u8, id: i32, count| Action::Drop(Throw { count, source: slot(Container::Hotbar, from, id), randomly: false });
    h.receive(1, &request(-3, vec![throw(0, sword.stack_id.unwrap(), 1)]));
    assert_eq!(answer(h.tick_all().to(1)).0, Status::Error);
    h.receive(1, &request(-5, vec![throw(4, diamonds.stack_id.unwrap(), 5)]));
    let sent = h.tick_all();
    assert_eq!(answer(sent.to(1)), (Status::Ok, vec![(Container::Hotbar, 4, 0, 0)]));
    let lying: Vec<AddItemEntity> = [1, 2].iter().map(|conn| all::<AddItemEntity>(sent.to(*conn))[0].clone()).collect();
    assert_eq!((lying[0].item.count, lying[0].position.z, lying[0].runtime_entity_id), (5, 2.0, lying[1].runtime_entity_id));
    assert_eq!(all::<MobEquipment>(sent.to(2))[0].item.network_id, 0, "the hand is empty now");

    // Nobody stands on them; once they can be taken, Alex walks over and has them.
    assert!((0..45).all(|_| all::<TakeItemEntity>(h.tick_all().to(2)).is_empty()));
    h.receive(2, &input_at([0.5, 65.0, 1.5]));
    let sent = h.tick_all();
    let taken = &all::<TakeItemEntity>(sent.to(1))[0];
    assert_eq!((taken.runtime_entity_id, all::<TakeItemEntity>(sent.to(2)).len()), (lying[0].runtime_entity_id, 1));
    assert!(u64::from(taken.target) != steve.runtime_id, "Alex took them");
    assert_eq!(all::<InventoryContent>(sent.to(2))[0].input[0].count, 5);
    let gone: Vec<i64> = [1, 2].iter().flat_map(|conn| all::<RemoveEntity>(sent.to(*conn))).map(|remove| remove.entity_id_self).collect();
    assert_eq!(gone, [lying[0].runtime_entity_id as i64; 2]);
}
