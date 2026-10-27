//! Bundles numbered, filled and emptied as BDS answered (inventory/bundles.rs;
//! research/captures/bds-bundles.txt).

use crate::common;

use acacia_proto::packets::{InventoryContent, InventorySlot};
use acacia_proto::types::{ContainerSlotType as Container, FullContainerName, ItemStackResponsesItemStatus as Status, ItemV4Extra, StackRequestSlotInfo};
use common::requests::{answer, place, request, slot, take};
use common::stored::{enter_stored, floor_holding};
use common::{Harness, all, config};
use sapling_world::{ItemStack, PlayerData, World, WorldStorage};

fn stack(name: &str, count: u8) -> Option<ItemStack> {
    Some(ItemStack { name: name.into(), count, damage: 0, block: None, tag: None })
}

fn in_bundle(id: u32, at: u8, stack_id: i32) -> StackRequestSlotInfo {
    StackRequestSlotInfo { slot_type: FullContainerName { container_id: Container::Dynamic, dynamic_container_id: Some(id) }, slot: at, stack_id }
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
    assert_eq!(numbered, [Some(0), Some(1)]);

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
