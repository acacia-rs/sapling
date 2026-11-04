//! Blocks a click with an item uses: a pot takes a plant, a campfire takes food, a brewing stand
//! shows its bottles.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddItemEntity, BlockEntityData, ContainerOpen, UpdateBlock};
use acacia_proto::types::{ContainerSlotType as Container, ItemStackResponsesItemStatus as Status};
use common::requests::{answer, place, request, slot};
use common::{Harness, all, click};
use sapling_api::{Block, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
const AT: [i32; 3] = [1, 65, 0];

/// Alex in survival with `item` in the hand beside the block `block`, Steve watching.
fn at(block: &'static str, item: Item) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(move |ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&item)));
        assert!(ctx.set_block(AT, &Block::new(block)));
    });
    h.tick_all();
    h
}

fn held(h: &mut Harness) -> Option<Item> {
    let seen = Arc::new(Mutex::new(None));
    let out = seen.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.held_item(ALEX));
    h.tick_all();
    seen.lock().unwrap().take()
}

fn plant(entity: &BlockEntityData) -> Option<Value> {
    entity.nbt.decode().unwrap().value.get("PlantBlock")?.get("name").cloned()
}

#[test]
fn a_pot_takes_a_plant_and_gives_it_back_to_an_empty_hand() {
    let mut h = at("flower_pot", Item::new("minecraft:poppy", 2));
    h.receive(1, &click(AT, 1, None));
    let shown = all::<BlockEntityData>(h.tick_all().to(2));
    assert_eq!(shown.iter().map(plant).collect::<Vec<_>>(), [Some(Value::String("minecraft:poppy".into()))]);
    assert_eq!(held(&mut h).map(|item| item.count), Some(1));

    h.receive(1, &click(AT, 1, None));
    assert!(all::<BlockEntityData>(h.tick_all().to(2)).is_empty(), "a pot holds one plant");

    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), None)));
    h.tick_all();
    h.receive(1, &click(AT, 1, None));
    let shown = all::<BlockEntityData>(h.tick_all().to(2));
    assert_eq!(shown.iter().map(plant).collect::<Vec<_>>(), [None]);
    assert_eq!(held(&mut h).map(|item| (item.name, item.count)), Some(("minecraft:poppy".to_owned(), 1)));
}

#[test]
fn a_potted_plant_keeps_its_first_state_as_on_bds() {
    let byte = |value| Value::Byte(value);
    let text = |value: &str| Value::String(value.into());
    // research/captures/bds-potsapling.txt, the world's pots read with block_entity_dump.
    let cases = [
        ("minecraft:oak_sapling", vec![("age_bit", byte(0))]),
        ("minecraft:mangrove_propagule", vec![("hanging", byte(0)), ("propagule_stage", Value::Int(0))]),
        ("minecraft:bamboo", vec![("age_bit", byte(0)), ("bamboo_leaf_size", text("no_leaves")), ("bamboo_stalk_thickness", text("thin"))]),
        ("minecraft:cactus", vec![("age", Value::Int(0))]),
    ];
    for (name, states) in cases {
        let mut h = at("flower_pot", Item::new(name, 1));
        h.receive(1, &click(AT, 1, None));
        let shown = all::<BlockEntityData>(h.tick_all().to(2));
        let kept = shown.first().and_then(|entity| entity.nbt.decode().unwrap().value.get("PlantBlock")?.get("states").cloned());
        let states = Value::Compound(states.into_iter().map(|(key, value)| (key.into(), value)).collect());
        assert_eq!(kept, Some(states), "{name}");
    }
}

#[test]
fn a_bottle_put_into_a_brewing_stand_shows_on_it() {
    let mut h = at("brewing_stand", Item::new("minecraft:potion", 1));
    h.receive(1, &click(AT, 1, None));
    assert!(!all::<ContainerOpen>(h.tick_all().to(1)).is_empty());
    h.receive(1, &request(-3, vec![place(1, slot(Container::Hotbar, 0, 0), slot(Container::BrewingResult, 1, 0))]));
    let sent = h.tick_all();
    assert_eq!(answer(sent.to(1)).0, Status::Ok);
    let registry = sapling_world::BlockRegistry::vanilla();
    let states: Vec<_> = all::<UpdateBlock>(sent.to(2)).iter().filter_map(|update| registry.get(registry.runtime_id_from_hash(update.block_runtime_id)?)).map(|state| state.properties).collect();
    assert_eq!(states, ["brewing_stand_slot_a_bit=1,brewing_stand_slot_b_bit=0,brewing_stand_slot_c_bit=0"]);
}

#[test]
fn food_on_a_lit_campfire_drops_off_cooked() {
    let mut h = at("campfire", Item::new("minecraft:beef", 3));
    h.receive(1, &click(AT, 1, None));
    let shown = all::<BlockEntityData>(h.tick_all().to(2));
    // The first is the set campfire's, shown again a tick on (bds-s7ents.txt).
    assert!(shown.last().is_some_and(|shown| shown.nbt.decode().unwrap().value.get("Item1").is_some()), "{shown:?}");
    assert_eq!(held(&mut h).map(|item| item.count), Some(2));
    let cooked_beef = sapling_data::item(&sapling_data::Stack { name: "minecraft:cooked_beef", count: 1, damage: 0, block: None, tag: None }, None).unwrap();
    let dropped: Vec<_> = (0..600).flat_map(|_| all::<AddItemEntity>(h.tick_all().to(2))).collect();
    assert_eq!(dropped.iter().map(|item| item.item.network_id).collect::<Vec<_>>(), [cooked_beef.network_id]);
}
