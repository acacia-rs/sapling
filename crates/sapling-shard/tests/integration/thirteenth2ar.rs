//! Group AR: a mob's AddEntity carries what it was made as, with nothing of it told again in its first tick
//! (research/captures/bds-ar2look.txt, bds-f3convert.txt; mobs/folded.rs).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEventEventId as Event, PlayerAuthInput, SetEntityData};
use acacia_proto::types::{InputData, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Meta, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use common::raising::{click_on, feet_of, has_flag, showed};
use common::shooting::{ALEX, FEET, ready, spawn, watch};
use common::stored::{enter_stored, floor_holding};
use common::{Harness, all, click, config, input_at};
use sapling_api::{GameMode, Item, Slot};
use sapling_world::World;

fn harness() -> Harness {
    let mut h = Harness::with(config(), World::with_storage(floor_holding(Vec::new(), &[])));
    enter_stored(&mut h, 1);
    h
}

/// What the tick a mob of `kind` appeared in told of it: its AddEntity and the items of every
/// SetEntityData about it but its first data (the long one BDS sends too).
fn appeared(sent: &[RawPacket], kind: &str) -> (AddEntity, Vec<Key>) {
    let added = all::<AddEntity>(sent).into_iter().find(|added| added.entity_type == kind).expect("shown");
    let told = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == added.runtime_id && data.metadata.len() < 8);
    (added.clone(), told.flat_map(|data| data.metadata.into_iter().map(|item| item.key)).collect())
}

fn value(added: &AddEntity, key: Key) -> Option<Meta> {
    added.metadata.iter().find(|item| item.key == key).map(|item| item.value.clone())
}

/// bds-ar2look.txt: a summoned armorer's AddEntity says `Variant` 8 and its skin, nothing after it does.
#[test]
fn a_summoned_armorer_is_announced_as_an_armorer() {
    let mut h = harness();
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:villager_v2", [2.5, 64.0, 0.5], 0.0, 0.0, Some("minecraft:become_armorer")).is_some()));
    let sent: Vec<_> = (0..2).flat_map(|_| h.tick(1)).collect();
    let (added, told) = appeared(&sent, "minecraft:villager_v2");
    assert_eq!(value(&added, Key::Variant), Some(Meta::Default(Plain::Int(8))));
    assert!(told.iter().all(|key| *key == Key::Flags), "only its flags follow: {told:?}");
}

/// bds-f3convert.txt: a villager killed by a zombie on hard comes back a zombie villager whose AddEntity
/// carries its profession, skin and tiers; no SetEntityData of that tick repeats them.
#[test]
fn a_turned_villager_is_announced_with_what_it_was() {
    let mut h = harness();
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:villager_v2", [2.5, 64.0, 0.5], 0.0, 0.0, Some("minecraft:become_armorer")).is_some()));
    let sent: Vec<_> = (0..3).flat_map(|_| h.tick(1)).collect();
    let villager = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:villager_v2").expect("shown");
    let skin = all::<SetEntityData>(&sent).into_iter().filter(|data| data.runtime_entity_id == villager.runtime_id).flat_map(|data| data.metadata).chain(villager.metadata.clone()).find(|item| item.key == Key::SkinId).map(|item| item.value);
    let id = villager.runtime_id;
    h.remote.run(move |ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Hard);
        let zombie = sapling_api::Attacker::Mob { kind: "minecraft:zombie".into() };
        assert!(ctx.hurt_entity_by(sapling_api::EntityId(id), 100.0, sapling_api::DamageCause::Attack, &zombie));
    });
    let sent = h.tick(1);
    let (added, told) = appeared(&sent, "minecraft:zombie_villager_v2");
    assert_eq!(value(&added, Key::Variant), Some(Meta::Default(Plain::Int(8))));
    assert_eq!(value(&added, Key::MaxTradeTier), Some(Meta::Default(Plain::Int(4))));
    assert_eq!(value(&added, Key::SkinId), skin);
    assert!(![Key::Variant, Key::SkinId, Key::MaxTradeTier, Key::TradeTier].iter().any(|key| told.contains(key)), "told again: {told:?}");
}

/// bds-ar2look.txt: a zombie horse's rider is announced in its seat and gets no first data, only its
/// flags; the horse gets one.
#[test]
fn a_jockeys_rider_gets_no_first_data() {
    let mut h = harness();
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:zombie_horse", [2.5, 64.0, 0.5], 0.0, 0.0, Some("minecraft:spawn_adult_with_rider")).is_some()));
    let sent = h.tick(1);
    let added = all::<AddEntity>(&sent);
    let rider = added.iter().find(|added| added.entity_type == "minecraft:zombie").expect("its rider");
    let horse = added.iter().find(|added| added.entity_type == "minecraft:zombie_horse").expect("the horse");
    let longest = |id: u64| all::<SetEntityData>(&sent).into_iter().filter(|data| data.runtime_entity_id == id).map(|data| data.metadata.len()).max().unwrap_or(0);
    assert!(longest(rider.runtime_id) <= 1, "the rider's: {}", longest(rider.runtime_id));
    assert!(longest(horse.runtime_id) > 8, "the horse's first data");
}

/// Whether a `FlagsExtended` told of `id` had the fox's fleeing bit on (bds-ar2fox.txt).
fn fled(sent: &[RawPacket], id: u64) -> bool {
    all::<SetEntityData>(sent).iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| &data.metadata).any(|item| matches!(item.value, Meta::FlagsExtended(flags) if flags.0 & (1 << 22) != 0))
}

/// The id of a fox that appeared at `feet`, and what Alex was told over the `ticks` after.
fn fox_near(h: &mut Harness, feet: [f32; 3], ticks: usize) -> (u64, Vec<RawPacket>) {
    h.remote.run(move |ctx| assert!(ctx.spawn_entity("minecraft:fox", feet).is_some()));
    let sent = watch(h, ticks);
    let fox = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:fox").expect("the fox").runtime_id;
    (fox, sent)
}

/// bds-ar2fox.txt: a fox 2 from a standing player flees, its flag on, to further than it was; one
/// beside a sneaking player stays (bds-ar2sneak.txt).
#[test]
fn a_fox_flees_a_standing_player_and_not_a_sneaking_one() {
    let mut h = ready(GameMode::Survival, &[], 0.0);
    let (fox, sent) = fox_near(&mut h, [2.5, 65.0, 0.5], 40);
    assert!(fled(&sent, fox), "its flag");
    let at = feet_of(&mut h, fox);
    assert!((at[0] - FEET[0]).hypot(at[2] - FEET[2]) > 2.5, "further off: {at:?}");

    let mut h = ready(GameMode::Survival, &[], 0.0);
    h.receive(1, &PlayerAuthInput { input_data: vec![InputData::StartSneaking], ..input_at(FEET) });
    let (fox, sent) = fox_near(&mut h, [2.5, 65.0, 0.5], 40);
    assert!(!fled(&sent, fox), "a sneaking player is not fled");
}

/// bds-ar2cat.txt: a cat tamed by feeding sits, told with its tamed flag; stood up, it goes to its
/// sleeping owner's bed and lies on its foot (`RESTING`), and gets up with the owner.
#[test]
fn a_tamed_cat_sits_and_lies_on_its_owners_bed() {
    let (mut h, cat) = cat_on_owners_bed();
    h.remote.run(|ctx| ctx.teleport(ALEX, FEET));
    assert_eq!(has_flag(&watch(&mut h, 3), cat, Flags::RESTING), Some(false), "up with its owner");
}

/// Alex asleep at 13000 in a bed by [`FEET`] with the cat he tamed lying on its foot; the cat's id.
pub(crate) fn cat_on_owners_bed() -> (Harness, u64) {
    let mut h = ready(GameMode::Survival, &[("minecraft:cod", 64, 0)], 0.0);
    let cat = spawn(&mut h, "minecraft:cat", [3.5, 65.0, 3.5]);
    let tamed = (0..60).map(|_| click_on(&mut h, cat)).find(|sent| showed(sent, cat, Event::TameSuccess)).expect("a third of cod tame");
    let flags = all::<SetEntityData>(&tamed).into_iter().filter(|data| data.runtime_entity_id == cat).find_map(|data| data.metadata.into_iter().find_map(|item| if let Meta::Flags(flags) = item.value { Some(flags) } else { None }));
    assert!(flags.is_some_and(|flags| flags.0 & Flags::TAMED.0 != 0 && flags.0 & Flags::SITTING.0 != 0), "sat as it was tamed: {flags:?}");
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:bed", 1)))));
    watch(&mut h, 1);
    assert_eq!(has_flag(&click_on(&mut h, cat), cat, Flags::SITTING), Some(false), "stood up");
    h.receive(1, &click([1, 64, 0], 1, None));
    h.remote.run(|ctx| ctx.set_time(13_000));
    watch(&mut h, 2);
    h.receive(1, &click([1, 65, 0], 1, None));
    let night = watch(&mut h, 100);
    assert_eq!(has_flag(&night, cat, Flags::RESTING), Some(true), "lying down");
    let at = feet_of(&mut h, cat);
    assert!((at[0] - 1.5).hypot(at[2] - 0.5) < 0.5 && at[1] > 65.5, "on the bed's foot: {at:?}");
    (h, cat)
}

/// bds-ar2look.txt: a cow summoned `entity_born` is announced a baby, at its scale and box.
#[test]
fn a_born_cow_is_announced_at_its_baby_scale() {
    let mut h = harness();
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:cow", [2.5, 64.0, 0.5], 0.0, 0.0, Some("minecraft:entity_born")).is_some()));
    let sent = h.tick(1);
    let (added, told) = appeared(&sent, "minecraft:cow");
    let (_, scale) = sapling_data::mob_look("minecraft:cow", true).expect("a recorded baby");
    assert_eq!(value(&added, Key::Scale), Some(Meta::Default(Plain::Float(scale))));
    assert!(!told.contains(&Key::Scale) && !told.contains(&Key::BoundingboxWidth), "told again: {told:?}");
}
