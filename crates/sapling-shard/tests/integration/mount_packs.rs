//! A tamed mount's inventory (riding/packs.rs, docs/research/mob-life.md "Mount inventories"):
//! a chest put on a donkey, and its screen opened by its rider's inventory key and by a sneaking click.

use std::sync::{Arc, Mutex};

use crate::common;
use crate::riding::{interact, spawned};

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, ContainerOpen, Interact, InventoryContent, InteractActionId, LevelSoundEvent, MobArmorEquipment, SetEntityData, UpdateEquipment};
use acacia_proto::types::{
    InputData, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, WindowType,
};
use common::{Harness, all, input_at};
use sapling_api::{EntityId, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

fn ticks(h: &mut Harness, count: usize) -> Vec<RawPacket> {
    (0..count).flat_map(|_| h.tick(1)).collect()
}

fn riders(h: &mut Harness, of: EntityId) -> usize {
    let count = Arc::new(Mutex::new(0));
    let slot = count.clone();
    h.remote.run(move |ctx| *slot.lock().unwrap() = ctx.riders(of).len());
    h.tick(1);
    let riders = *count.lock().unwrap();
    riders
}

fn ask(action_id: InteractActionId, target: u64) -> Interact {
    Interact { action_id, target_entity_id: target, has_position: false, position: None }
}

/// The horse screens Alex was shown in `sent`: the mob each names and how many slots it has.
/// The horse screens opened, as (mob, slots): BDS sends `UpdateEquipment` alone, size 0, and then
/// the window's content (research/captures/bds-tack.txt).
fn screens(sent: &[RawPacket]) -> Vec<(i64, u8)> {
    assert!(all::<ContainerOpen>(sent).is_empty(), "no ContainerOpen");
    let contents = all::<InventoryContent>(sent);
    let equipped = all::<UpdateEquipment>(sent).into_iter().filter(|told| told.window_type == WindowType::Horse && told.size == 0);
    equipped.filter_map(|told| contents.iter().find(|content| format!("{:?}", content.window_id) == format!("{:?}", told.window_id)).map(|content| (told.entity_id, content.input.len() as u8))).collect()
}

#[test]
fn a_tamed_donkey_opens_to_its_rider_takes_a_chest_and_opens_to_a_sneaking_click() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let donkey = spawned(&mut h, &[("minecraft:donkey", [2.5, 65.0, 0.5])])[0];
    let tamed = (0..25).any(|_| {
        h.receive(1, &interact(donkey.0));
        ticks(&mut h, 2);
        riders(&mut h, donkey) == 1
    });
    assert!(tamed, "a ride tamed it");
    h.receive(1, &ask(InteractActionId::OpenInventory, 1));
    let sent = ticks(&mut h, 2);
    assert_eq!(screens(&sent), [(donkey.0 as i64, 2)], "its saddle and armour slots");

    h.receive(1, &ask(InteractActionId::LeaveVehicle, donkey.0));
    ticks(&mut h, 2);
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:chest", 1)))));
    h.tick(1);
    h.receive(1, &interact(donkey.0));
    let sent = ticks(&mut h, 2);
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "add.chest"));
    h.remote.run(move |ctx| {
        assert!(ctx.item(ALEX, Slot::Main(0)).is_none(), "the chest went on");
        assert!(ctx.riders(donkey).is_empty(), "a chest put on is all the click does");
    });
    h.tick(1);

    let sent = sneaking_click(&mut h, donkey);
    assert_eq!(screens(&sent), [(donkey.0 as i64, 17)], "tack and 15 chest slots");
    assert_eq!(riders(&mut h, donkey), 0, "a sneaking click opens it and does not ride");
}

/// Alex sneaks, then clicks `mob`.
fn sneaking_click(h: &mut Harness, mob: EntityId) -> Vec<RawPacket> {
    let feet = Arc::new(Mutex::new([0.0; 3]));
    let slot = feet.clone();
    h.remote.run(move |ctx| *slot.lock().unwrap() = ctx.position(ALEX).expect("Alex").feet);
    h.tick(1);
    let feet = *feet.lock().unwrap();
    let mut input = input_at(feet);
    // Getting off moved Alex: the client owns up to that teleport first.
    input.input_data = vec![InputData::HandledTeleport, InputData::StartSneaking];
    h.receive(1, &input);
    h.tick(1);
    h.receive(1, &interact(mob.0));
    ticks(h, 2)
}

/// Alex rides `mob` until it is tamed, then gets off.
fn tame_by_riding(h: &mut Harness, mob: EntityId) {
    let tamed = (0..25).any(|_| {
        h.receive(1, &interact(mob.0));
        ticks(h, 2);
        riders(h, mob) == 1
    });
    assert!(tamed, "a ride tamed it");
    h.receive(1, &ask(InteractActionId::LeaveVehicle, mob.0));
    ticks(h, 2);
}

/// Alex clicks `mob` holding `item`.
fn click_holding(h: &mut Harness, mob: EntityId, item: &'static str) -> Vec<RawPacket> {
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(item, 1)))));
    h.tick(1);
    h.receive(1, &interact(mob.0));
    ticks(h, 2)
}

#[test]
fn a_llama_shows_its_strength_wears_a_carpet_and_spills_its_chest_but_not_its_carpet_when_it_dies() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let made = Arc::new(Mutex::new(None));
    let slot = made.clone();
    h.remote.run(move |ctx| *slot.lock().unwrap() = ctx.spawn_entity("minecraft:llama", [2.5, 65.0, 0.5]));
    let sent = ticks(&mut h, 2);
    let llama = made.lock().unwrap().expect("a llama");
    let told = all::<SetEntityData>(&sent).into_iter().filter(|data| data.runtime_entity_id == llama.0).flat_map(|data| data.metadata);
    let strength = told
        .filter(|item| item.key == Key::Strength)
        .find_map(|item| match item.value {
            Value::Default(Plain::Int(strength)) => Some(strength),
            _ => None,
        })
        .expect("its strength told");
    assert!((1..=5).contains(&strength), "{strength}");

    tame_by_riding(&mut h, llama);
    let sent = click_holding(&mut h, llama, "minecraft:red_carpet");
    let carpet = sapling_data::item(&sapling_data::Stack { name: "minecraft:red_carpet", count: 1, damage: 0, block: None, tag: None }, None).expect("an item").network_id;
    assert!(all::<MobArmorEquipment>(&sent).iter().any(|worn| worn.runtime_entity_id == llama.0 && worn.body.network_id == carpet), "the carpet shows on it");
    click_holding(&mut h, llama, "minecraft:chest");
    let sent = sneaking_click(&mut h, llama);
    assert_eq!(screens(&sent), [(llama.0 as i64, 2 + 3 * strength as u8)], "tack and 3 slots a point of strength");

    h.remote.run(move |ctx| assert!(ctx.kill_entity(llama)));
    let sent = ticks(&mut h, 2);
    let mut left: Vec<&str> = all::<AddItemEntity>(&sent).iter().filter_map(|lying| sapling_data::item_name(lying.item.network_id.into())).collect();
    left.sort_unstable();
    // BDS's dying llama kept its carpet (research/captures/bds-tack.txt).
    assert!(!left.contains(&"minecraft:red_carpet") && left.contains(&"minecraft:chest"), "{left:?}");
}

#[test]
fn a_donkey_summoned_tame_opens_to_a_sneaking_click_even_holding_a_chest() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let made = Arc::new(Mutex::new(None));
    let slot = made.clone();
    h.remote.run(move |ctx| *slot.lock().unwrap() = ctx.summon_as(None, "minecraft:donkey", [2.5, 65.0, 0.5], 0.0, Some("minecraft:spawn_tame_adult")));
    ticks(&mut h, 2);
    let donkey = made.lock().unwrap().expect("a donkey");
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:chest", 1)))));
    let sent = sneaking_click(&mut h, donkey);
    // BDS's opened its screen and took no chest (research/captures/bds-tack.txt).
    assert!(!all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "add.chest"));
    assert_eq!(screens(&sent), [(donkey.0 as i64, 2)]);
}
