//! Bundles numbered, filled and emptied as BDS answered (inventory/bundles.rs;
//! research/captures/bds-bundles.txt).

use crate::common;

use acacia_proto::packets::{ContainerRegistryCleanup, InventoryContent, InventorySlot};
use acacia_proto::types::{ContainerSlotType as Container, FullContainerName, ItemStackResponsesItemStatus as Status, ItemV4Extra, StackRequestSlotInfo};
use common::requests::{answer, place, request, slot, take};
use common::stored::{enter_stored, floor_holding};
use common::{Harness, all, config};
use sapling_api::{PlayerId, Slot};
use sapling_world::{ItemStack, PlayerData, World, WorldStorage};

fn stack(name: &str, count: u8) -> Option<ItemStack> {
    Some(ItemStack { name: name.into(), count, damage: 0, block: None, tag: None })
}

fn in_bundle(id: u32, at: u8, stack_id: i32) -> StackRequestSlotInfo {
    StackRequestSlotInfo { slot_type: FullContainerName { container_id: Container::Dynamic, dynamic_container_id: Some(wire(id)) }, slot: at, stack_id }
}

/// A bundle's number as acacia-proto decodes BDS's little-endian one (bds-container-cleanup.txt).
fn wire(id: u32) -> u32 {
    id.swap_bytes()
}

#[test]
fn a_bundle_takes_up_to_its_weight_and_gives_back() {
    let items = vec![stack("minecraft:bundle", 1), stack("minecraft:cobblestone", 41), stack("minecraft:ender_pearl", 16), stack("minecraft:diamond_sword", 1), stack("minecraft:bundle", 1)];
    let storage = floor_holding(Vec::new(), &[]);
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &PlayerData { main: items, ..PlayerData::default() }).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage));
    let mut sent = enter_stored(&mut h, 1);
    (0..12).for_each(|_| sent.extend(h.tick(1)));
    let numbered: Vec<_> = all::<InventoryContent>(&sent).into_iter().filter(|content| content.container.container_id == Container::Dynamic).map(|content| content.container.dynamic_container_id).collect();
    assert_eq!(numbered, [Some(wire(0)), Some(wire(1))]);

    h.receive(1, &request(-1, vec![place(40, slot(Container::Hotbar, 1, 0), in_bundle(0, 0, 0))]));
    let placed = h.tick(1);
    let (status, slots) = answer(&placed);
    assert_eq!(status, Status::Ok);
    let cobble = slots.iter().find(|(container, ..)| *container == Container::Dynamic).copied().expect("the bundle's slot");
    assert_eq!(cobble.2, 40);
    let shown = all::<InventorySlot>(&placed);
    assert_eq!((shown.len(), shown[0].slot, shown[0].item.count), (1, 0, 40));
    // The bundle goes out without what it holds.
    let ItemV4Extra::Default(Some(extra)) = &shown[0].storage_item.as_ref().expect("the bundle").extra else { panic!() };
    assert!(format!("{:?}", extra.nbt).contains("bundle_id") && !format!("{:?}", extra.nbt).contains("storage_item"));

    h.receive(1, &request(-3, vec![place(4, slot(Container::Hotbar, 2, 0), in_bundle(0, 1, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
    // 40 + 4 * 4 = 56: one more cobblestone fits, a sword, a pearl or a bundle does not.
    for (id, from, count) in [(-5, 3, 1), (-7, 2, 3), (-9, 4, 1)] {
        h.receive(1, &request(id, vec![place(count, slot(Container::Hotbar, from, 0), in_bundle(0, 2, 0))]));
        assert_eq!(answer(&h.tick(1)), (Status::Unknown(55), Vec::new()), "hotbar {from}");
    }
    h.receive(1, &request(-11, vec![place(1, slot(Container::Hotbar, 1, 0), in_bundle(0, 2, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);

    h.receive(1, &request(-13, vec![take(5, in_bundle(0, 0, cobble.3), slot(Container::Hotbar, 5, 0))]));
    let taken = h.tick(1);
    assert!(answer(&taken).1.contains(&(Container::Dynamic, 0, 35, cobble.3)), "{:?}", answer(&taken).1);
    assert_eq!(all::<InventorySlot>(&taken)[0].item.count, 35);
}

/// BDS's `cleanup` drill (research/captures/bds-container-cleanup.txt): a cleared bundle's
/// container is let go at the next 15 s sweep, to every player.
#[test]
fn a_bundle_gone_is_let_go_at_the_next_sweep() {
    let storage = floor_holding(Vec::new(), &[]);
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &PlayerData { main: vec![stack("minecraft:bundle", 1), stack("minecraft:bundle", 1)], ..PlayerData::default() }).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage));
    enter_stored(&mut h, 1);
    h.enter(2, "Alex");
    (0..12).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert!(ctx.set_item(PlayerId(1), Slot::Main(1), None)));
    let told: Vec<(u64, Vec<FullContainerName>)> = (0..320)
        .flat_map(|_| {
            let sent = h.tick_all();
            [1, 2].map(|conn| (conn, all::<ContainerRegistryCleanup>(sent.to(conn)).into_iter().flat_map(|cleanup| cleanup.removed_containers).collect::<Vec<_>>()))
        })
        .filter(|(_, removed)| !removed.is_empty())
        .collect();
    let bundle_one = FullContainerName { container_id: Container::Dynamic, dynamic_container_id: Some(wire(1)) };
    assert_eq!(told, [(1, vec![bundle_one.clone()]), (2, vec![bundle_one])]);
}

#[test]
fn a_kept_bundle_shows_what_it_holds_after_a_rejoin_and_an_empty_slot_answers_49() {
    let storage = floor_holding(Vec::new(), &[]);
    let steve = acacia_auth::login::offline_identity("Steve");
    storage.store_player(&steve, &PlayerData { main: vec![stack("minecraft:bundle", 1), stack("minecraft:cobblestone", 41)], ..PlayerData::default() }).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    (0..12).for_each(|_| drop(h.tick(1)));
    h.receive(1, &request(-1, vec![place(10, slot(Container::Hotbar, 1, 0), in_bundle(0, 0, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
    h.leave(1);
    h.shard.tick();
    h.shard.save_worlds();
    drop(h);

    let mut h = Harness::with(config(), World::with_storage(storage));
    let mut sent = enter_stored(&mut h, 1);
    (0..3).for_each(|_| sent.extend(h.tick(1)));
    let shown: Vec<InventoryContent> = all::<InventoryContent>(&sent).into_iter().filter(|content| content.container.dynamic_container_id == Some(0)).collect();
    let [content] = &shown[..] else { panic!("{} contents of bundle 0", shown.len()) };
    assert_eq!((content.input.len(), content.input[0].count, content.input[1].count), (64, 10, 0));
    let id = content.input[0].stack_id.expect("a stack id");

    h.receive(1, &request(-3, vec![take(4, in_bundle(0, 0, id), slot(Container::Hotbar, 5, 0))]));
    assert!(answer(&h.tick(1)).1.contains(&(Container::Dynamic, 0, 6, id)), "taken under the id shown");
    h.receive(1, &request(-5, vec![take(1, in_bundle(0, 7, 0), slot(Container::Hotbar, 6, 0))]));
    let refused = h.tick(1);
    assert_eq!(answer(&refused), (Status::Unknown(49), Vec::new()));
    assert!(all::<InventoryContent>(&refused).is_empty(), "nothing resent");
}
