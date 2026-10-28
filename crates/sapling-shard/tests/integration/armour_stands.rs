//! Armour stands dressed and posed by players, against research/captures/bds-armorstand.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, LevelEvent, LevelEventEvent, LevelSoundEvent, MobArmorEquipment, RemoveEntity, SetEntityData};
use acacia_proto::types::{InputData, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, TransactionTransactionData};
use common::asking::ask;
use common::raising::use_on;
use common::{Harness, all, attack, input_at};
use sapling_api::{GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
const STAND: [f32; 3] = [2.5, 65.0, 0.5];

/// Alex holding `held` before an armour stand; the stand's runtime id.
fn before_a_stand(held: Option<(&'static str, u8)>) -> (Harness, u64) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), held.map(|(name, count)| Item::new(name, count)).as_ref()));
        assert!(ctx.spawn_entity("minecraft:armor_stand", STAND).is_some());
    });
    let shown: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let stand = all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type == "minecraft:armor_stand").expect("shown").runtime_id;
    (h, stand)
}

/// Alex clicks the stand `up` above its feet; what Alex was told.
fn click_at(h: &mut Harness, stand: u64, up: f32) -> Vec<RawPacket> {
    let mut click = use_on(stand);
    if let TransactionTransactionData::ItemUseOnEntity(on) = &mut click.transaction.transaction_data {
        (on.click_pos.x, on.click_pos.y, on.click_pos.z) = (STAND[0], STAND[1] + up, STAND[2]);
    }
    h.receive(1, &click);
    h.tick_all().to(1).to_vec()
}

fn held(h: &mut Harness) -> Option<(String, u8)> {
    ask(h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|held| (held.name, held.count)))
}

#[test]
fn a_helmet_goes_on_its_head_and_an_empty_hand_takes_it_back() {
    let (mut h, stand) = before_a_stand(Some(("minecraft:diamond_helmet", 1)));
    let told = click_at(&mut h, stand, 0.3);
    let worn = all::<MobArmorEquipment>(&told);
    assert!(worn.len() == 1 && worn[0].helmet.network_id != 0 && worn[0].boots.network_id == 0, "{worn:?}");
    let sounds: Vec<String> = all::<LevelSoundEvent>(&told).into_iter().map(|sound| sound.sound_id).collect();
    assert_eq!(sounds, ["armor.equip_diamond", "mob.armor_stand.place"]);
    assert_eq!(held(&mut h), None, "handed over in survival");

    assert!(all::<MobArmorEquipment>(&click_at(&mut h, stand, 0.3)).is_empty(), "nothing at its feet to take");
    let taken = all::<MobArmorEquipment>(&click_at(&mut h, stand, 1.8));
    assert!(taken.len() == 1 && taken[0].helmet.network_id == 0);
    assert_eq!(held(&mut h), Some(("minecraft:diamond_helmet".into(), 1)));
}

#[test]
fn one_of_a_stack_goes_in_its_hand_and_a_sneaking_click_turns_its_pose() {
    let (mut h, stand) = before_a_stand(Some(("minecraft:stick", 5)));
    click_at(&mut h, stand, 1.0);
    assert_eq!(held(&mut h), Some(("minecraft:stick".into(), 4)));
    let mut input = input_at([0.5, 65.0, 0.5]);
    input.input_data = vec![InputData::StartSneaking];
    h.receive(1, &input);
    h.tick_all();
    let posed = all::<SetEntityData>(&click_at(&mut h, stand, 1.0));
    let pose = posed.iter().flat_map(|data| &data.metadata).find(|item| item.key == Key::ArmorStandPoseIndex).map(|item| item.value.clone());
    assert_eq!(pose, Some(Value::Default(Plain::Int(1))));
    assert_eq!(held(&mut h), Some(("minecraft:stick".into(), 4)), "posing takes nothing");
}

/// Alex hits the stand; what Alex was told that tick and the `more` after.
fn hit(h: &mut Harness, stand: u64, more: usize) -> Vec<RawPacket> {
    h.receive(1, &attack(stand));
    (0..=more).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

fn ints(told: &[RawPacket], key: Key) -> Vec<i32> {
    let value = |item: &acacia_proto::types::MetadataDictionaryItem| match &item.value {
        Value::Default(Plain::Int(value)) if item.key == key => Some(*value),
        _ => None,
    };
    all::<SetEntityData>(told).iter().flat_map(|data| data.metadata.iter().filter_map(value).collect::<Vec<_>>()).collect()
}

fn events(told: &[RawPacket]) -> Vec<LevelEventEvent> {
    all::<LevelEvent>(told).into_iter().map(|event| event.event).filter(|event| format!("{event:?}").contains("ArmorStand")).collect()
}

#[test]
fn a_hit_wobbles_it_and_a_second_soon_after_knocks_it_over_dropping_it_and_its_gear() {
    // research/captures/bds-standhits.txt.
    let (mut h, stand) = before_a_stand(Some(("minecraft:diamond_helmet", 1)));
    click_at(&mut h, stand, 0.3);
    let told = hit(&mut h, stand, 9);
    assert_eq!(events(&told), [LevelEventEvent::SoundArmorStandHit]);
    assert_eq!((ints(&told, Key::Health), ints(&told, Key::HurtDirection)), (vec![-10], vec![-1]));
    assert_eq!(ints(&told, Key::HurtTime), (0..=9).rev().collect::<Vec<_>>());
    (0..30).for_each(|_| _ = h.tick_all());
    let told = hit(&mut h, stand, 1);
    assert_eq!((events(&told), ints(&told, Key::Health), ints(&told, Key::HurtDirection)), (vec![LevelEventEvent::SoundArmorStandHit], vec![], vec![1]), "two seconds on, only a wobble");
    h.receive(1, &attack(stand));
    let told: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!(events(&told), [LevelEventEvent::SoundArmorStandHit, LevelEventEvent::ParticleDestroyArmorStand, LevelEventEvent::SoundArmorStandBreak]);
    assert_eq!((all::<RemoveEntity>(&told).len(), all::<AddItemEntity>(&told).len()), (1, 2), "gone, leaving itself and its helmet");
}

#[test]
fn a_creative_hit_knocks_it_over_leaving_nothing() {
    let (mut h, stand) = before_a_stand(None);
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    h.tick_all();
    let told = hit(&mut h, stand, 2);
    assert_eq!(events(&told), [LevelEventEvent::ParticleDestroyArmorStand, LevelEventEvent::SoundArmorStandBreak]);
    assert_eq!((all::<RemoveEntity>(&told).len(), all::<AddItemEntity>(&told).len()), (1, 0));
}
