//! Group AX's explorer maps (docs/research/villagers.md "Explorer maps"): a bought map is made
//! again once it reaches the inventory.

use crate::common;
use crate::villager_days::village;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{ClientboundMapItemData, InventorySlot, UpdateTrade};
use acacia_proto::types::{ContainerSlotType as Container, ItemStackRequestActionsItemContent as Action, ItemStackRequestActionsItemContentCraftRecipe as Craft, ItemStackResponsesItemStatus as Status};
use common::all;
use common::requests::{answer, consume, place, request, slot, take};
use sapling_api::{Item, PlayerId};

const ALEX: PlayerId = PlayerId(1);

fn int(offer: &Value, key: &str) -> i32 {
    if let Some(Value::Int(value)) = offer.get(key) { *value } else { panic!("{key}") }
}

#[test]
fn a_bought_explorer_map_gets_a_new_id_told_after_the_offers() {
    // research/captures/bds-at1maps.txt: the offer's map (13) told whole, the slot shown with a new
    // map_uuid, then the new map (47) told whole; nothing of either told when the offers rolled.
    let (mut h, id) = village(2_000, [4.5, 65.0, 4.5], "minecraft:become_cartographer");
    common::asking::ask(&mut h, |ctx| {
        assert_eq!(ctx.give(ALEX, &Item::new("minecraft:emerald", 64)), 0);
        assert_eq!(ctx.give(ALEX, &Item::new("minecraft:compass", 1)), 0);
    });
    let shown = common::raising::click_on(&mut h, id);
    assert!(all::<ClientboundMapItemData>(&shown).is_empty(), "no map told with the offers");
    let screen = all::<UpdateTrade>(&shown).into_iter().next().expect("its screen");
    let offers = screen.offers.decode().unwrap().value;
    let Some(Value::List(recipes)) = offers.get("Recipes") else { panic!("no recipes") };
    let offer = recipes.items.iter().find(|recipe| recipe.get("sell").and_then(|sell| sell.get("Name")) == Some(&Value::String("minecraft:filled_map".into()))).expect("a map offer");
    let Some(Value::Long(offered)) = offer.get("sell").and_then(|sell| sell.get("tag")).and_then(|tag| tag.get("map_uuid")) else { panic!("map_uuid") };
    let offered = *offered;

    let emeralds = int(offer, "buyCountA") as u8;
    h.receive(1, &request(-1, vec![place(emeralds, slot(Container::Hotbar, 0, 0), slot(Container::Trade2Ingredient1, 4, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
    h.receive(1, &request(-3, vec![place(1, slot(Container::Hotbar, 1, 0), slot(Container::Trade2Ingredient2, 5, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
    let bought = vec![
        Action::CraftRecipe(Craft { recipe_network_id: int(offer, "netId") as u32, times_crafted: 1 }),
        consume(emeralds, slot(Container::Trade2Ingredient1, 4, 0)),
        consume(1, slot(Container::Trade2Ingredient2, 5, 0)),
        take(1, slot(Container::CreativeOutput, 50, 0), slot(Container::Cursor, 0, 0)),
    ];
    h.receive(1, &request(-5, bought));
    let sent = h.tick(1);
    assert_eq!(answer(&sent).0, Status::Ok);
    assert!(all::<ClientboundMapItemData>(&sent).is_empty(), "nothing told while it is on the cursor");

    h.receive(1, &request(-7, vec![place(1, slot(Container::Cursor, 0, 0), slot(Container::Hotbar, 1, 0))]));
    let sent: Vec<_> = (0..2).flat_map(|_| h.tick(1)).collect();
    let maps: Vec<i64> = all::<ClientboundMapItemData>(&sent).iter().filter(|map| map.pixels.is_some()).map(|map| map.map_id).collect();
    assert_eq!(maps.len(), 2, "{maps:?}");
    assert_eq!(maps[0], offered);
    assert_ne!(maps[1], offered, "a new map");
    assert!(all::<InventorySlot>(&sent).iter().any(|shown| shown.slot == 1), "its slot shown");
}
