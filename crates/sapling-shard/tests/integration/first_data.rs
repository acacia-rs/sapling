//! The sixth wave's entity data (research/captures/bds-firstdata.txt, bds-mountdeath.txt): an
//! item's first data less what is as made, and a horse-family mob's open mouth at a hurt and at
//! its death.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::{DamageCause, EntityId, Item, MobSlot};

fn summon(h: &mut Harness, kind: &'static str, event: Option<&'static str>) -> EntityId {
    ask(h, move |ctx| ctx.summon_as(None, kind, [2.5, 65.0, 2.5], 0.0, 0.0, event)).expect(kind)
}

fn ticks(h: &mut Harness, n: usize) -> Vec<RawPacket> {
    (0..n).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

/// The values of `key` told of `id`, in order.
fn told(sent: &[RawPacket], id: EntityId, key: Key) -> Vec<Value> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id.0).flat_map(|data| data.metadata).filter(|item| item.key == key).map(|item| item.value).collect()
}

const OPEN: Value = Value::Default(Plain::Long(128));
const SHUT: Value = Value::Default(Plain::Long(0));

#[test]
fn a_hurt_horse_opens_its_mouth_for_29_ticks_and_at_its_death_a_llama_never() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let horse = summon(&mut h, "minecraft:horse", Some("minecraft:spawn_tame_adult"));
    let llama = summon(&mut h, "minecraft:llama", Some("minecraft:spawn_tame_adult"));
    ticks(&mut h, 2);
    h.remote.run(move |ctx| assert!(ctx.hurt_entity(horse, 2.0, DamageCause::Generic) && ctx.hurt_entity(llama, 2.0, DamageCause::Generic)));
    let each: Vec<_> = (0..35).map(|_| ticks(&mut h, 1)).collect();
    let mouth: Vec<(usize, Value)> = each.iter().enumerate().flat_map(|(tick, sent)| told(sent, horse, Key::MinecartDisplayBlock).into_iter().map(move |value| (tick, value))).collect();
    let ok = mouth.len() == 2 && mouth[0].1 == OPEN && mouth[1].1 == SHUT && mouth[0].0 <= 1 && mouth[1].0 - mouth[0].0 == 29;
    assert!(ok, "{mouth:?}");
    let hurt = each.concat();
    assert_eq!(ask(&mut h, move |ctx| ctx.set_mob_item(horse, MobSlot::Saddle, Some(&Item::new("minecraft:saddle", 1)))), Ok(()));
    ticks(&mut h, 1);
    h.remote.run(move |ctx| assert!(ctx.kill_entity(horse) && ctx.kill_entity(llama)));
    let died = ticks(&mut h, 2);
    let data: Vec<_> = all::<SetEntityData>(&died).into_iter().filter(|data| data.runtime_entity_id == horse.0).collect();
    // bds-mountdeath.txt: one packet, its saddle's flags gone, its mouth open.
    let keys: Vec<_> = data.iter().map(|data| data.metadata.iter().map(|item| item.key).collect::<Vec<_>>()).collect();
    assert_eq!(keys, [vec![Key::Flags, Key::MinecartDisplayBlock]]);
    let Value::Flags(flags) = data[0].metadata[0].value else { panic!("a flag word") };
    assert_eq!(flags.0 & (MetadataFlags1::SADDLED.0 | MetadataFlags1::WASD_CONTROLLED.0), 0);
    for sent in [&hurt, &died] {
        assert!(told(sent, llama, Key::MinecartDisplayBlock).is_empty(), "a llama has no such mouth");
    }
}

#[test]
fn an_item_s_first_data_leaves_out_what_every_actor_is_made_with() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let cow = summon(&mut h, "minecraft:cow", None);
    ticks(&mut h, 2);
    h.remote.run(move |ctx| assert!(ctx.kill_entity(cow)));
    let sent = ticks(&mut h, 2);
    let item = all::<AddItemEntity>(&sent).into_iter().next().expect("its loot");
    let first = all::<SetEntityData>(&sent).into_iter().find(|data| data.runtime_entity_id == item.runtime_entity_id).expect("its first data");
    let keys: Vec<_> = first.metadata.iter().map(|item| item.key).collect();
    assert!(!keys.contains(&Key::Scale) && !keys.contains(&Key::RiderSeatPosition) && keys.contains(&Key::BoundingboxWidth), "{keys:?}");
    assert_eq!(item.metadata.len(), first.metadata.len() + 5, "bds-firstdata.txt: 64 announced, 59 again");
}
