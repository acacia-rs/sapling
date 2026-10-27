//! A cartography table zooming, locking, copying and renaming a map (inventory/forge/cartography.rs,
//! maps.rs; research/captures/bds-maps.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{ClientboundMapItemData, ContainerOpen, ItemStackResponse};
use acacia_proto::types::{
    ContainerSlotType as Container, ItemStackRequestActionsItemContent as Action, ItemStackRequestActionsItemContentOptional as Optional, ItemStackRequestCause,
    ItemStackResponsesItemStatus as Status, WindowType,
};
use common::requests::{answer, consume, place, request, slot};
use common::shooting::click_air;
use common::stored::{enter_stored, floor_holding};
use common::{Harness, all, click, config};
use sapling_world::{ItemStack, PlayerData, World, WorldStorage};

const TABLE: [i32; 3] = [1, 64, 1];
const ZOOM: &str = "8b36268c-1829-483c-a0f1-993b7156a8f2";
const LOCK: &str = "602234e4-cac1-4353-8bb7-b1ebff70024b";
const COPY: &str = "442d85ed-8272-4543-a6f1-418f90ded05d";

fn stack(name: &str, count: u8) -> Option<ItemStack> {
    Some(ItemStack { name: name.into(), count, damage: 0, block: None, tag: None })
}

/// Steve, who filled the empty map in his first slot, at the table's open screen.
fn at_table(items: Vec<Option<ItemStack>>) -> (Harness, ClientboundMapItemData) {
    let storage = floor_holding(Vec::new(), &[(TABLE, "minecraft:cartography_table")]);
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &PlayerData { main: items, ..PlayerData::default() }).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage));
    enter_stored(&mut h, 1);
    (0..12).for_each(|_| drop(h.tick(1)));
    h.receive(1, &click_air());
    let map = all::<ClientboundMapItemData>(&h.tick(1)).pop().expect("the map made");
    h.receive(1, &click(TABLE, 1, None));
    assert_eq!(all::<ContainerOpen>(&h.tick(1))[0].window_type, WindowType::Cartography);
    (h, map)
}

/// Puts hotbar `map` and `added` (if any) in the table and crafts by the recipe `uuid` (0 for none),
/// naming the result `name`; the result goes to hotbar `to`.
fn craft(h: &mut Harness, map: u8, added: Option<u8>, uuid: Option<&str>, name: &str, to: u8) -> Vec<RawPacket> {
    let mut puts = vec![place(1, slot(Container::Hotbar, map, 0), slot(Container::CartographyInput, 12, 0))];
    puts.extend(added.map(|from| place(1, slot(Container::Hotbar, from, 0), slot(Container::CartographyAdditional, 13, 0))));
    for (i, put) in puts.into_iter().enumerate() {
        h.receive(1, &request(-1 - 2 * i as i32, vec![put]));
        assert_eq!(answer(&h.tick(1)).0, Status::Ok);
    }
    let recipe_network_id = uuid.map_or(0, |uuid| sapling_data::multi_recipe(uuid).expect("the recipe"));
    let mut actions = vec![Action::Optional(Optional { recipe_network_id, filtered_string_index: 0 }), consume(1, slot(Container::CartographyInput, 12, 0))];
    actions.extend(added.map(|_| consume(1, slot(Container::CartographyAdditional, 13, 0))));
    let made = if uuid == Some(COPY) { 2 } else { 1 };
    actions.push(place(made, slot(Container::CreativeOutput, 50, 0), slot(Container::HotbarAndInventory, to, 0)));
    let mut asked = request(-9, actions);
    (asked.requests[0].custom_names, asked.requests[0].cause) = (vec![name.to_owned()], ItemStackRequestCause::CartographyText);
    h.receive(1, &asked);
    h.tick(1)
}

#[test]
fn a_map_zooms_out_locks_copies_and_takes_a_name() {
    let items = vec![stack("minecraft:empty_map", 1), stack("minecraft:paper", 1), stack("minecraft:glass_pane", 1), stack("minecraft:empty_map", 1)];
    let (mut h, map) = at_table(items);

    let zoomed = craft(&mut h, 0, Some(1), Some(ZOOM), "", 0);
    assert_eq!(answer(&zoomed).0, Status::Ok);
    let zoom = all::<ClientboundMapItemData>(&zoomed).into_iter().find(|packet| packet.map_id == map.map_id + 1).expect("the zoomed map, whole");
    // Centred anew on the 256-block square around the old centre: BDS's (256, -256) became (320, -192).
    let centre = ((map.origin.x + 64) as f64 / 256.0).floor() as i32 * 256 + 64;
    assert_eq!((zoom.scale, zoom.origin.x, zoom.width), (Some(1), centre, Some(128)));
    assert!(zoom.pixels.unwrap().iter().all(|pixel| *pixel == 0));

    let locked = craft(&mut h, 0, Some(2), Some(LOCK), "", 0);
    assert_eq!(answer(&locked).0, Status::Ok);
    let lock = all::<ClientboundMapItemData>(&locked).into_iter().find(|packet| packet.locked).expect("the locked map, whole");
    assert!(lock.map_id != map.map_id + 1 && lock.scale == Some(1));

    let copied = craft(&mut h, 0, Some(3), Some(COPY), "", 0);
    assert_eq!(answer(&copied).1.last().map(|(_, _, count, _)| *count), Some(2));

    let named = craft(&mut h, 0, None, None, "Named", 1);
    assert_eq!(answer(&named).0, Status::Ok);
    let slots = &all::<ItemStackResponse>(&named)[0].responses[0];
    let names: Vec<String> = slots.containers.iter().flatten().flat_map(|container| &container.slots).map(|slot| slot.custom_name.clone()).collect();
    assert!(names.contains(&"Named".to_owned()), "{names:?}");
}
