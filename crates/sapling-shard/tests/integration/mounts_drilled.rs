//! The still-open wave's mounts (research/captures/bds-mountflags.txt, bds-mountslots.txt,
//! bds-mountdeath.txt): a summon naming an event unheard, a saddle's control flags, a chested
//! donkey's `slot.chest` bound, a mob's command-held hands saved, a dying mount's saddle and a
//! llama's announced strength; and the third wave's (bds-saddleevents.txt, bds-saddlerule.txt): the
//! saddling spawn events, a pig's saddle on death, and which mounts take tack by command.

use crate::common;
use crate::riding::interact;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, LevelSoundEvent, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use common::asking::ask;
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_api::{Difficulty, EntityId, Item, MobSlot, MobSlotRefusal, PlayerId, ShardCtx, Slot};
use sapling_world::{ChunkPos, World, WorldStorage, stored_actors};

const ALEX: PlayerId = PlayerId(1);

fn alex() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h
}

/// What `call` returned on the shard's next tick, and what that tick and the next sent Alex.
fn told<T: Send + 'static>(h: &mut Harness, call: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> (T, Vec<RawPacket>) {
    let answer = std::sync::Arc::new(std::sync::Mutex::new(None));
    let put = answer.clone();
    h.remote.run(move |ctx| *put.lock().unwrap() = Some(call(ctx)));
    let mut sent = h.tick_all().to(1).to_vec();
    sent.extend(h.tick_all().to(1).to_vec());
    (answer.lock().unwrap().take().expect("the shard ran the call"), sent)
}

fn summoned(h: &mut Harness, kind: &'static str, event: Option<&'static str>, z: f32) -> (EntityId, Vec<RawPacket>) {
    ask(h, move |ctx| ctx.set_difficulty(Difficulty::Easy));
    let (id, sent) = told(h, move |ctx| ctx.summon_as(None, kind, [4.5, 65.0, z], 0.0, event));
    (id.expect("summoned"), sent)
}

fn put(h: &mut Harness, id: EntityId, slot: MobSlot, item: Option<&'static str>) -> (Result<(), MobSlotRefusal>, Vec<RawPacket>) {
    told(h, move |ctx| ctx.set_mob_item(id, slot, item.map(|item| Item::new(item, 1)).as_ref()))
}

fn flags(sent: &[RawPacket], id: EntityId) -> Vec<i64> {
    let items = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id.0).flat_map(|data| data.metadata);
    items.filter_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0) } else { None }).collect()
}

fn int(items: &[MetadataDictionaryItem], key: Key) -> Option<i32> {
    items.iter().find(|item| item.key == key).and_then(|item| if let Value::Default(Plain::Int(value)) = item.value { Some(value) } else { None })
}

#[test]
fn a_summon_naming_an_event_is_not_heard_spawning() {
    let mut h = alex();
    let heard = |sent: &[RawPacket]| all::<LevelSoundEvent>(sent).iter().any(|sound| sound.sound_id == "spawn");
    let (_, sent) = summoned(&mut h, "minecraft:horse", Some("minecraft:spawn_tame_adult"), 0.5);
    assert!(!heard(&sent), "BDS's 112 summons with an event were silent");
    let (_, sent) = summoned(&mut h, "minecraft:horse", None, 4.5);
    assert!(heard(&sent));
}

#[test]
fn a_saddle_shows_its_mount_s_controls_and_air_leaves_it_on() {
    let mut h = alex();
    let (horse, _) = summoned(&mut h, "minecraft:horse", Some("minecraft:spawn_tame_adult"), 0.5);
    let (saddled, sent) = put(&mut h, horse, MobSlot::Saddle, Some("minecraft:saddle"));
    assert_eq!(saddled, Ok(()));
    let horse_controls = Flags::SADDLED.0 | Flags::WASD_CONTROLLED.0 | Flags::CAN_POWER_JUMP.0;
    assert!(flags(&sent, horse).last().is_some_and(|word| word & horse_controls == horse_controls), "{:?}", flags(&sent, horse));
    let (kept, sent) = put(&mut h, horse, MobSlot::Saddle, None);
    assert_eq!((kept, flags(&sent, horse)), (Ok(()), vec![]), "air answered success and changed nothing");

    let (camel, _) = summoned(&mut h, "minecraft:camel", Some("minecraft:spawn_adult"), 4.5);
    let (saddled, sent) = put(&mut h, camel, MobSlot::Saddle, Some("minecraft:saddle"));
    assert_eq!(saddled, Ok(()));
    let camel_controls = Flags::SADDLED.0 | Flags::WASD_CONTROLLED.0 | Flags::CAN_DASH.0;
    let word = flags(&sent, camel).last().copied().unwrap_or_default();
    assert!(word & camel_controls == camel_controls && word & Flags::CAN_POWER_JUMP.0 == 0, "{word:b}");
}

#[test]
fn slot_chest_refuses_a_donkey_s_last_goods_slot_as_bds_did() {
    let mut h = alex();
    let (donkey, _) = summoned(&mut h, "minecraft:donkey", Some("minecraft:spawn_tame_adult"), 0.5);
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:chest", 1)))));
    h.tick(1);
    h.receive(1, &interact(donkey.0));
    (0..2).for_each(|_| drop(h.tick(1)));
    assert_eq!(put(&mut h, donkey, MobSlot::Chest(13), Some("minecraft:diamond")).0, Ok(()));
    assert_eq!(put(&mut h, donkey, MobSlot::Chest(14), Some("minecraft:diamond")).0, Err(MobSlotRefusal::BadSlot { size: 15 }));
    assert_eq!(put(&mut h, donkey, MobSlot::Inventory(15), Some("minecraft:coal")).0, Ok(()));
    assert_eq!(put(&mut h, donkey, MobSlot::Inventory(16), Some("minecraft:coal")).0, Err(MobSlotRefusal::BadSlot { size: 16 }));
}

#[test]
fn hands_given_by_command_are_saved_with_the_mob() {
    let storage = floor_with(Vec::new());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    let zombie = ask(&mut h, |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.summon_as(None, "minecraft:zombie", [8.5, 64.0, 8.5], 0.0, None)
    })
    .expect("a zombie");
    assert_eq!(put(&mut h, zombie, MobSlot::MainHand, Some("minecraft:iron_sword")).0, Ok(()));
    assert_eq!(put(&mut h, zombie, MobSlot::OffHand, Some("minecraft:shield")).0, Ok(()));
    h.shard.save_worlds();
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let saved = stored_actors(&column.actors).unwrap().into_iter().find_map(|(actor, ..)| actor.filter(|actor| actor.identifier == "minecraft:zombie")).expect("saved");
    let name = |held: &Option<sapling_world::ItemStack>| held.as_ref().map(|held| held.name.clone());
    assert_eq!((name(&saved.mainhand), name(&saved.offhand)), (Some("minecraft:iron_sword".into()), Some("minecraft:shield".into())));
}

#[test]
fn a_dying_mount_lets_go_of_its_saddle_but_not_its_armour() {
    let mut h = alex();
    let (horse, _) = summoned(&mut h, "minecraft:horse", Some("minecraft:spawn_tame_adult"), 0.5);
    let (camel, _) = summoned(&mut h, "minecraft:camel", Some("minecraft:spawn_adult"), 6.5);
    for (mob, slot, item) in [(horse, MobSlot::Saddle, "minecraft:saddle"), (horse, MobSlot::Body, "minecraft:iron_horse_armor"), (camel, MobSlot::Saddle, "minecraft:saddle")] {
        assert_eq!(put(&mut h, mob, slot, Some(item)).0, Ok(()), "{item}");
    }
    let (_, sent) = told(&mut h, move |ctx| ctx.kill_entity(horse) && ctx.kill_entity(camel));
    let left: Vec<_> = all::<AddItemEntity>(&sent).iter().filter_map(|lying| sapling_data::item_name(lying.item.network_id.into())).collect();
    // research/captures/bds-mountdeath.txt
    assert_eq!(left.iter().filter(|name| **name == "minecraft:saddle").count(), 2, "{left:?}");
    assert!(!left.contains(&"minecraft:iron_horse_armor"), "{left:?}");
}

#[test]
fn a_llama_is_announced_with_its_own_strength_and_three_slots_a_point() {
    let mut h = alex();
    let (llama, sent) = summoned(&mut h, "minecraft:llama", Some("minecraft:spawn_tame_adult"), 0.5);
    let added = all::<AddEntity>(&sent).into_iter().find(|added| added.runtime_id == llama.0).expect("announced");
    let data: Vec<_> = all::<SetEntityData>(&sent).into_iter().filter(|data| data.runtime_entity_id == llama.0).flat_map(|data| data.metadata).collect();
    let strength = int(&data, Key::Strength).expect("its strength told after");
    // research/captures/bds-mountslots.txt: BDS's announcement already held both.
    assert_eq!((int(&added.metadata, Key::Strength), int(&added.metadata, Key::ContainerExtraSlotsPerStrength)), (Some(strength), Some(3)));
    assert_eq!(int(&data, Key::ContainerExtraSlotsPerStrength), Some(3));
}

fn left_on_death(h: &mut Harness, mob: EntityId) -> Vec<&'static str> {
    let (_, sent) = told(h, move |ctx| ctx.kill_entity(mob));
    all::<AddItemEntity>(&sent).iter().filter_map(|lying| sapling_data::item_name(lying.item.network_id.into())).collect()
}

#[test]
fn a_saddling_spawn_event_shows_the_saddle_and_only_a_pig_s_drops() {
    let mut h = alex();
    let saddle = |sent: &[RawPacket]| all::<LevelSoundEvent>(sent).iter().any(|sound| sound.sound_id == "saddle");
    let (horse, sent) = summoned(&mut h, "minecraft:horse", Some("minecraft:horse_saddled"), 0.5);
    let controls = Flags::SADDLED.0 | Flags::WASD_CONTROLLED.0 | Flags::CAN_POWER_JUMP.0;
    assert!(flags(&sent, horse).last().is_some_and(|word| word & controls == controls), "{:?}", flags(&sent, horse));
    assert!(saddle(&sent), "the event's play_sound");
    // research/captures/bds-saddleevents.txt: the horse family's event holds no saddle, and with no
    // adult group the mob has no loot (a zombie horse's is 2 to 3 rotten flesh).
    assert_eq!(left_on_death(&mut h, horse), Vec::<&str>::new());
    let (zombie_horse, _) = summoned(&mut h, "minecraft:zombie_horse", Some("minecraft:horse_saddled"), 8.5);
    assert_eq!(left_on_death(&mut h, zombie_horse), Vec::<&str>::new());

    let (pig, sent) = summoned(&mut h, "minecraft:pig", Some("minecraft:on_saddled"), 4.5);
    assert!(flags(&sent, pig).last().is_some_and(|word| word & Flags::SADDLED.0 != 0), "{:?}", flags(&sent, pig));
    assert!(!saddle(&sent), "a pig's event is unheard");
    assert_eq!(left_on_death(&mut h, pig).first(), Some(&"minecraft:saddle"), "BDS's saddle came first");
}

#[test]
fn slot_saddle_takes_only_a_grown_tamed_mount_and_never_a_skeleton_horse() {
    let mut h = alex();
    // research/captures/bds-saddlerule.txt
    let cases = [
        ("minecraft:horse", Some("minecraft:spawn_adult"), Err(MobSlotRefusal::NoSlot)),
        ("minecraft:horse", Some("minecraft:entity_born"), Err(MobSlotRefusal::NoSlot)),
        ("minecraft:donkey", Some("minecraft:spawn_adult"), Err(MobSlotRefusal::NoSlot)),
        ("minecraft:zombie_horse", Some("minecraft:spawn_adult"), Err(MobSlotRefusal::NoSlot)),
        ("minecraft:zombie_horse", Some("minecraft:spawn_tame_adult"), Ok(())),
        ("minecraft:skeleton_horse", None, Err(MobSlotRefusal::NoSlot)),
        ("minecraft:camel", Some("minecraft:entity_born"), Err(MobSlotRefusal::NoSlot)),
        ("minecraft:pig", None, Err(MobSlotRefusal::NoSlot)),
    ];
    for (index, (kind, event, answer)) in cases.into_iter().enumerate() {
        let (mob, _) = summoned(&mut h, kind, event, index as f32 * 3.0 + 0.5);
        assert_eq!(put(&mut h, mob, MobSlot::Saddle, Some("minecraft:saddle")).0, answer, "{kind} {event:?}");
    }
}
