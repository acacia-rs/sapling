//! A brewing stand brews: a blaze powder for fuel, a brew of 400 ticks shown on its screen, the
//! bottles turned by BDS's mixes (docs/research/brewing.md).

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::{ContainerOpen, ContainerSetData, InventorySlot, LevelSoundEvent};
use acacia_proto::types::{ContainerSlotType as Container, ItemStackResponsesItemStatus as Status};
use common::requests::{answer, place, request, slot};
use common::{Harness, all, click};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
const AT: [i32; 3] = [1, 65, 0];

/// Alex in survival beside a brewing stand holding `items` from the first slot, its screen open.
fn open(items: &[(&'static str, u8, i16)]) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let items: Vec<Item> = items.iter().map(|&(name, count, damage)| Item { damage, ..Item::new(name, count) }).collect();
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Survival);
        for (place, item) in items.iter().enumerate() {
            assert!(ctx.set_item(ALEX, Slot::Main(place as u8), Some(item)));
        }
        assert!(ctx.set_block(AT, &Block::new("brewing_stand")));
    });
    h.tick_all();
    h.receive(1, &click(AT, 1, None));
    assert!(!all::<ContainerOpen>(h.tick_all().to(1)).is_empty());
    h
}

/// Puts `count` from hotbar slot `from` into the stand's slot `to`; the answer.
fn put(h: &mut Harness, id: i32, from: u8, to: u8, count: u8) -> Status {
    let kind = match to {
        0 => Container::BrewingInput,
        4 => Container::BrewingFuel,
        _ => Container::BrewingResult,
    };
    h.receive(1, &request(id, vec![place(count, slot(Container::Hotbar, from, 0), slot(kind, to, 0))]));
    answer(h.tick_all().to(1)).0
}

fn held(h: &mut Harness, place: u8) -> Option<Item> {
    let seen = Arc::new(Mutex::new(None));
    let out = seen.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.item(ALEX, Slot::Main(place)));
    h.tick_all();
    seen.lock().unwrap().take()
}

#[test]
fn water_and_wart_brew_into_awkward_potions_in_twenty_seconds() {
    let mut h = open(&[("minecraft:blaze_powder", 2, 0), ("minecraft:nether_wart", 1, 0), ("minecraft:potion", 1, 0), ("minecraft:potion", 1, 0)]);
    assert_eq!(put(&mut h, -3, 0, 4, 1), Status::Ok);
    assert_eq!(put(&mut h, -5, 2, 1, 1), Status::Ok);
    assert_eq!(put(&mut h, -7, 3, 2, 1), Status::Ok);
    assert_eq!(put(&mut h, -9, 1, 0, 1), Status::Ok);
    let mut sent = Vec::new();
    for _ in 0..402 {
        sent.extend(h.tick_all().to(1).to_vec());
    }
    let data = all::<ContainerSetData>(&sent);
    let value = |property: i32| data.iter().filter(|set| set.property == property).map(|set| set.value).collect::<Vec<_>>();
    assert_eq!(value(1), [19], "the finished brew took one of the powder's twenty, told when it went in");
    // The 400 went with the answer to the put.
    assert!(value(0).first() == Some(&399) && value(0).last() == Some(&0), "{:?}", &value(0)[..5.min(value(0).len())]);
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "potion.brewed"));
    let slots = all::<InventorySlot>(&sent);
    assert!(slots.iter().any(|shown| shown.slot == 1 && shown.item.metadata == 4), "an awkward potion");
    assert_eq!(held(&mut h, 0).map(|item| item.count), Some(1), "one powder left in the hand");
}

#[test]
fn a_stand_refuses_what_it_does_not_brew_with() {
    let mut h = open(&[("minecraft:diamond", 1, 0), ("minecraft:stick", 1, 0)]);
    assert_ne!(put(&mut h, -3, 0, 0, 1), Status::Ok, "a diamond is no ingredient");
    assert_ne!(put(&mut h, -5, 1, 4, 1), Status::Ok, "a stick is no fuel");
}
