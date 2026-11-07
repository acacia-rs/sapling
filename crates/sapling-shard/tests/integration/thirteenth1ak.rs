//! Against the group AK drills: a baby turtle's scute (research/captures/bds-ak1scute.txt) and an
//! allay handed an item (bds-ak1allay.txt).

use std::sync::{Arc, Mutex};

use crate::common;

use acacia_proto::packets::{AddItemEntity, LevelSoundEvent, MobEquipment, SetEntityData};
use acacia_proto::types::MetadataDictionaryItemKey as Key;
use common::raising::click_on;
use common::shooting::{ALEX, FEET, held, ready, spawn, watch};
use common::{Harness, all};
use sapling_api::{EntityId, GameMode, Slot};

fn network_id(item: &str) -> i16 {
    sapling_data::item(&sapling_data::Stack { name: item, count: 1, damage: 0, block: None, tag: None }, None).expect("an item").network_id as i16
}

/// A `kind` summoned with `event` at `feet`; its runtime id.
fn summon_as(h: &mut Harness, kind: &'static str, feet: [f32; 3], event: &'static str) -> u64 {
    let id = Arc::new(Mutex::new(None));
    let out = id.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.summon_as(None, kind, feet, 0.0, 0.0, Some(event)));
    watch(h, 3);
    let EntityId(id) = id.lock().unwrap().take().expect("summoned");
    id
}

#[test]
fn a_baby_turtle_fed_ten_seagrass_drops_a_scute_from_its_feet_as_it_grows_up() {
    // bds-ak1scute.txt: grown at the tenth seagrass, the scute (one) from the feet + 0.125 (the item box's
    // middle), tossed 0.2 up.
    let mut h = ready(GameMode::Creative, &[("minecraft:seagrass", 64, 0)], 0.0);
    let feet = [FEET[0] + 2.0, FEET[1], FEET[2]];
    let turtle = summon_as(&mut h, "minecraft:turtle", feet, "minecraft:entity_born");
    let mut told: Vec<_> = (0..10).flat_map(|_| click_on(&mut h, turtle)).collect();
    told.extend(watch(&mut h, 2));
    let scute = network_id("minecraft:turtle_scute");
    let shed: Vec<AddItemEntity> = all::<AddItemEntity>(&told).into_iter().filter(|item| item.item.network_id == scute).collect();
    assert_eq!(shed.len(), 1, "one scute");
    assert_eq!(shed[0].item.count, 1);
    assert!((shed[0].position.y - (feet[1] + 0.125)).abs() < 0.05 && (shed[0].velocity.y - 0.2).abs() < 1e-4, "{:?} {:?}", shed[0].position, shed[0].velocity);
}

#[test]
fn an_allay_takes_one_of_the_held_stack_and_is_heard_and_owned_a_tick_later() {
    // bds-ak1allay.txt: 4 dirt in hand, the allay took 1 (MobEquipment count 1) and the hand kept 3 in that
    // tick; the next, `item_given` and its flags with OwnerEid; a second click with dirt did nothing.
    let mut h = ready(GameMode::Survival, &[("minecraft:dirt", 4, 0)], 0.0);
    let allay = spawn(&mut h, "minecraft:allay", [FEET[0] + 1.0, FEET[1], FEET[2]]);
    let given = click_on(&mut h, allay);
    let hands: Vec<MobEquipment> = all::<MobEquipment>(&given).into_iter().filter(|held| held.runtime_entity_id == allay).collect();
    assert!(hands.len() == 1 && hands[0].item.count == 1 && hands[0].item.network_id == network_id("minecraft:dirt"), "{hands:?}");
    assert!(all::<LevelSoundEvent>(&given).iter().all(|sound| sound.sound_id != "item_given"), "heard a tick later");
    let next = watch(&mut h, 1);
    assert!(all::<LevelSoundEvent>(&next).iter().any(|sound| sound.sound_id == "item_given"));
    assert!(all::<SetEntityData>(&next).iter().any(|data| data.runtime_entity_id == allay && data.metadata.iter().any(|item| item.key == Key::OwnerEid)));
    assert_eq!(held(&mut h, Slot::Main(0)).map(|dirt| dirt.count), Some(3));
    let again = click_on(&mut h, allay);
    assert!(all::<MobEquipment>(&again).iter().all(|held| held.runtime_entity_id != allay), "holding dirt, it takes no more");
    assert_eq!(held(&mut h, Slot::Main(0)).map(|dirt| dirt.count), Some(3));
    common::asking::ask(&mut h, |ctx| ctx.set_item(ALEX, Slot::Main(0), None));
    let taken = click_on(&mut h, allay);
    assert!(all::<MobEquipment>(&taken).iter().any(|held| held.runtime_entity_id == allay && held.item.network_id == 0), "its hand emptied");
    assert_eq!(held(&mut h, Slot::Main(0)).map(|dirt| dirt.count), Some(1), "the dirt back");
}
