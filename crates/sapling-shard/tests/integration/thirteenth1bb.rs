//! Group BB's `/event entity` (research/captures/bds-bb1event.txt): an event fires only by the name the
//! mob's definition gives it, and a villager's `become_<profession>` gives it that profession's offers.

use crate::common;
use crate::villager_days::village;

use acacia_proto::nbt::Value;
use acacia_proto::packets::UpdateTrade;
use common::all;
use sapling_api::EntityId;

/// What the villager `id` offers when clicked: its screen's title and every offer's sold item.
fn offers(h: &mut common::Harness, id: u64) -> (String, Vec<String>) {
    let shown = common::raising::click_on(h, id);
    let screen = all::<UpdateTrade>(&shown).into_iter().next().expect("its screen");
    let offers = screen.offers.decode().unwrap().value;
    let Some(Value::List(recipes)) = offers.get("Recipes") else { panic!("no recipes") };
    let sold = recipes.items.iter().filter_map(|recipe| match recipe.get("sell").and_then(|sell| sell.get("Name")) {
        Some(Value::String(name)) => Some(name.to_string()),
        _ => None,
    });
    (screen.display_name.to_string(), sold.collect())
}

#[test]
fn an_evented_villager_takes_the_profession_its_event_names_with_its_offers() {
    let (mut h, id) = village(2_000, [4.5, 65.0, 4.5], "minecraft:become_farmer");
    assert_eq!(offers(&mut h, id).0, "entity.villager.farmer");
    // bds-bb1event.txt: the pack's names only, `become_librarian` has none; the shepherd's is spelt `sheperd`.
    let fired = common::asking::ask(&mut h, move |ctx| ["become_librarian", "minecraft:become_zombie", "minecraft:become_sheperd"].map(|event| ctx.fire_entity_event(EntityId(id), event)));
    assert_eq!(fired, [false, false, true]);
    let (title, sold) = offers(&mut h, id);
    assert_eq!(title, "entity.villager.shepherd");
    assert!(sold.iter().any(|item| item == "minecraft:shears") && !sold.iter().any(|item| item == "minecraft:bread"), "{sold:?}");
    assert!(common::asking::ask(&mut h, move |ctx| ctx.fire_entity_event(EntityId(id), "minecraft:become_cartographer")));
    let (title, sold) = offers(&mut h, id);
    assert_eq!(title, "entity.villager.cartographer");
    assert!(sold.iter().any(|item| item == "minecraft:empty_map"), "{sold:?}");
}
