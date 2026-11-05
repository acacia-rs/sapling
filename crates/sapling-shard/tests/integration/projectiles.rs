//! A player's bow, crossbow and trident (docs/research/projectiles.md).

use crate::common;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{LevelSoundEvent, SetEntityMotion, TakeItemEntity};
use acacia_proto::types::{MetadataDictionaryItemValue, MetadataFlags1};
use acacia_proto::RawPacket;
use common::shooting::{ALEX, added, draw, face, healths, held, ready, spawn, throw, watch};
use common::stored::{actor, enchanted, enter_stored, floor_with, plain};
use common::{Harness, all, config};
use sapling_api::{GameMode, Item, PlayerId, Slot, Weather};
use sapling_world::{ChunkPos, ItemStack, PlayerData, World, WorldStorage};

const BOW: (&str, u8, i16) = ("minecraft:bow", 1, 0);
const ARROWS: (&str, u8, i16) = ("minecraft:arrow", 2, 0);
const COW: [f32; 3] = [0.5, 65.0, 3.5];

fn is_critical(arrow: &acacia_proto::packets::AddEntity) -> bool {
    arrow.metadata.iter().any(|item| matches!(item.value, MetadataDictionaryItemValue::Flags(flags) if flags.0 & MetadataFlags1::CRITICAL.0 != 0))
}

fn arrows_held(h: &mut Harness) -> Option<u8> {
    held(h, Slot::Main(1)).map(|arrows| arrows.count)
}

#[test]
fn a_half_drawn_bow_costs_a_cow_three_and_spends_an_arrow() {
    let mut h = ready(GameMode::Survival, &[BOW, ARROWS], 5.0);
    spawn(&mut h, "minecraft:cow", COW);
    let sent = draw(&mut h, 10, 10);
    let arrow = added(&sent, "minecraft:arrow");
    assert_eq!(arrow.len(), 1);
    assert!(!is_critical(&arrow[0]), "not a full draw");
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "bow"));
    // A draw of 10 ticks looses at 1.25: twice its speed, rounded up.
    assert_eq!(healths(&sent), [7.0]);
    assert_eq!(arrows_held(&mut h), Some(1));
}

/// research/captures/bds-r1shots.txt: AddEntity, `bow`, then the arrow's `disappeared` a block
/// above where it starts; `reappeared` from its middle two ticks on.
#[test]
fn a_loosed_arrow_is_announced_before_the_bow_is_heard_and_disappears_until_its_second_tick() {
    let mut h = ready(GameMode::Survival, &[BOW, ARROWS], 5.0);
    let sent = draw(&mut h, 20, 5);
    let arrow = added(&sent, "minecraft:arrow");
    let id = arrow[0].unique_id;
    let at = |name: &str| sent.iter().position(|packet| packet.decode::<LevelSoundEvent>().is_ok_and(|sound| sound.sound_id == name)).unwrap_or_else(|| panic!("{name} heard"));
    let announced = sent.iter().position(|packet| packet.is::<acacia_proto::packets::AddEntity>()).expect("announced");
    assert!(announced < at("bow") && at("bow") < at("disappeared") && at("disappeared") < at("reappeared"));
    let sounds = all::<LevelSoundEvent>(&sent);
    let gone = sounds.iter().find(|sound| sound.sound_id == "disappeared").unwrap();
    assert_eq!((gone.entity_type.as_str(), gone.entity_unique_id, gone.position.y), ("minecraft:arrow", id, arrow[0].position.y + 1.0));
    assert!(sounds.iter().any(|sound| sound.sound_id == "reappeared" && sound.entity_unique_id == id));
}

#[test]
fn a_draw_too_short_looses_nothing_and_a_full_one_is_critical() {
    let mut h = ready(GameMode::Survival, &[BOW, ARROWS], 5.0);
    assert!(added(&draw(&mut h, 1, 5), "minecraft:arrow").is_empty());
    assert_eq!(arrows_held(&mut h), Some(2));
    let arrow = added(&draw(&mut h, 20, 0), "minecraft:arrow");
    assert!(arrow.len() == 1 && is_critical(&arrow[0]));
}

#[test]
fn a_survival_arrow_is_picked_up_again_and_a_creative_one_given_nobody() {
    let mut h = ready(GameMode::Survival, &[BOW, ARROWS], 80.0);
    let sent = draw(&mut h, 10, 15);
    assert_eq!((added(&sent, "minecraft:arrow").len(), all::<TakeItemEntity>(&sent).len()), (1, 1));
    assert_eq!(arrows_held(&mut h), Some(2), "back in the inventory");

    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    let sent = draw(&mut h, 10, 15);
    assert_eq!(all::<TakeItemEntity>(&sent).len(), 1, "taken away");
    assert_eq!(arrows_held(&mut h), Some(2), "neither spent nor given");
}

/// research/captures/bds-f1pickup.txt: a stuck arrow is to be had 8 ticks after it lay still.
#[test]
fn a_stuck_arrow_is_picked_up_once_it_stops_shaking() {
    use acacia_proto::packets::SetEntityMotion;
    let mut h = ready(GameMode::Survival, &[BOW, ARROWS], 80.0);
    let loosed = draw(&mut h, 10, 0);
    let ticks: Vec<Vec<RawPacket>> = std::iter::once(loosed).chain((0..20).map(|_| h.tick_all().to(1).to_vec())).collect();
    let stopped = ticks.iter().position(|sent| all::<SetEntityMotion>(sent).iter().any(|told| [told.velocity.x, told.velocity.y, told.velocity.z] == [0.0; 3]));
    let taken = ticks.iter().position(|sent| !all::<TakeItemEntity>(sent).is_empty());
    assert_eq!(taken.zip(stopped).map(|(taken, stopped)| taken - stopped), Some(8), "lay still at {stopped:?}, taken at {taken:?}");
}

/// Steve with `main` in a stored world of `actors`, survival.
fn stored(main: Vec<Option<ItemStack>>, actors: Vec<u8>) -> (Harness, std::sync::Arc<sapling_world::MemoryStorage>) {
    let storage = floor_with(actors);
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &PlayerData { main, ..PlayerData::default() }).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    (0..12).for_each(|_| drop(h.tick(1)));
    h.remote.run(|ctx| ctx.set_game_mode(PlayerId(1), GameMode::Survival));
    (h, storage)
}

#[test]
fn a_stuck_arrow_is_kept_with_its_column_and_lies_there_again() {
    let (mut h, storage) = stored(vec![plain("minecraft:bow"), Some(ItemStack { count: 2, ..plain("minecraft:arrow").unwrap() })], Vec::new());
    face(&mut h, 1, PlayerId(1), [0.5, 64.0, 0.5], 30.0);
    assert_eq!(added(&draw(&mut h, 10, 20), "minecraft:arrow").len(), 1);
    h.shard.save_worlds();
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().unwrap();
    assert!(column.actors.windows(15).any(|bytes| bytes == b"minecraft:arrow"), "kept with the column");

    // A creative player's arrow a survival one leaves; a survival one's it takes.
    let creative = actor(-5, "minecraft:arrow", [4.5, 64.0, 0.5], ("isCreative", Value::Byte(1)));
    let shot = actor(-6, "minecraft:arrow", [2.5, 64.0, 4.5], ("player", Value::Byte(1)));
    let (mut h, _) = stored(Vec::new(), [creative, shot].concat());
    let left = [face(&mut h, 1, PlayerId(1), [4.5, 64.0, 0.5], 0.0), watch(&mut h, 5)].concat();
    assert!(all::<TakeItemEntity>(&left).is_empty());
    let taken = [face(&mut h, 1, PlayerId(1), [2.5, 64.0, 4.5], 0.0), watch(&mut h, 5)].concat();
    assert_eq!(all::<TakeItemEntity>(&taken).len(), 1);
    assert_eq!(held(&mut h, Slot::Main(0)), Some(Item::new("minecraft:arrow", 1)));
}

#[test]
fn infinity_spends_no_plain_arrow() {
    let (mut h, _) = stored(vec![enchanted("minecraft:bow", &[(22, 1)], 0), Some(ItemStack { count: 2, ..plain("minecraft:arrow").unwrap() })], Vec::new());
    face(&mut h, 1, PlayerId(1), [0.5, 64.0, 0.5], 5.0);
    assert_eq!(added(&draw(&mut h, 10, 2), "minecraft:arrow").len(), 1);
    assert_eq!(arrows_held(&mut h), Some(2));
}

#[test]
fn a_crossbow_loads_in_25_ticks_and_shoots_three_with_multishot() {
    let (mut h, _) = stored(vec![enchanted("minecraft:crossbow", &[(33, 1)], 0), Some(ItemStack { count: 3, ..plain("minecraft:arrow").unwrap() })], Vec::new());
    face(&mut h, 1, PlayerId(1), [0.5, 64.0, 0.5], 5.0);
    draw(&mut h, 24, 1);
    assert_eq!(arrows_held(&mut h), Some(3), "let go too soon");
    draw(&mut h, 25, 1);
    assert_eq!(arrows_held(&mut h), Some(2), "loaded");
    let sent = throw(&mut h, 1);
    let arrows = added(&sent, "minecraft:arrow");
    assert!(arrows.len() == 3 && arrows.iter().all(is_critical), "{}", arrows.len());
    let sideways: Vec<f32> = arrows.iter().map(|arrow| arrow.velocity.x).collect();
    assert!(sideways[0] * sideways[2] < 0.0 && sideways[1].abs() < sideways[0].abs(), "a side one, the straight one, the other side (bds-t2emulti.txt): {sideways:?}");
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "crossbow.shoot"));
    assert!(added(&throw(&mut h, 1), "minecraft:arrow").is_empty(), "the next use loads it again");
}

#[test]
fn a_thrown_trident_costs_a_cow_eight_and_leaves_the_hand() {
    let mut h = ready(GameMode::Survival, &[("minecraft:trident", 1, 0)], 5.0);
    spawn(&mut h, "minecraft:cow", COW);
    let sent = draw(&mut h, 10, 10);
    assert_eq!(added(&sent, "minecraft:thrown_trident").len(), 1);
    assert_eq!(healths(&sent), [2.0]);
    assert_eq!(held(&mut h, Slot::Main(0)), None);
}

/// research/captures/bds-rainramp.txt: a riptide trident threw BDS's actor 2-3 s after `/weather
/// clear`, the rain still falling, and did nothing once it had stopped.
#[test]
fn a_riptide_throws_only_in_the_rain_still_falling() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let trident = Item { enchantments: vec![(30, 1)], ..Item::new("minecraft:trident", 1) };
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&trident))));
    let thrown = |sent: Vec<RawPacket>| all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "item.trident.riptide_1");
    assert!(!thrown(draw(&mut h, 10, 2)), "dry");
    h.remote.run(|ctx| ctx.set_weather(Weather::Rain, None));
    watch(&mut h, 100);
    h.remote.run(|ctx| ctx.set_weather(Weather::Clear, None));
    watch(&mut h, 40);
    assert!(thrown(draw(&mut h, 10, 2)), "the rain still falling 50 ticks after it cleared");
    watch(&mut h, 60);
    assert!(!thrown(draw(&mut h, 10, 2)), "the rain gone");
}

/// bds-r1shots.txt: BDS heard `item.trident.riptide_3` and sent the thrower no SetEntityMotion; the
/// client launches itself.
#[test]
fn a_riptide_launch_sends_its_thrower_no_motion() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let trident = Item { enchantments: vec![(30, 3)], ..Item::new("minecraft:trident", 1) };
    h.remote.run(move |ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&trident)));
        ctx.set_weather(Weather::Rain, None);
    });
    watch(&mut h, 100);
    let sent = draw(&mut h, 10, 2);
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "item.trident.riptide_3"), "launched");
    assert!(all::<SetEntityMotion>(&sent).is_empty());
}

/// `LightningRodBlock`'s projectile hit (Linux 0xc538e10): in a thunderstorm a Channeling trident on
/// a rod calls a bolt to the middle of the rod's cell, with no `item.trident.thunder`, and powers
/// it (research/captures/bds-m3rod.txt; this asked for the cell above and the sound before).
#[test]
fn a_channeling_trident_on_a_rod_calls_a_bolt_onto_it_in_a_storm() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let trident = Item { enchantments: vec![(32, 1)], ..Item::new("minecraft:trident", 1) };
    h.remote.run(move |ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&trident)));
        // At the height a level throw meets it; one above would shut out the sky.
        ctx.set_block([0, 66, 4], &sapling_api::Block::new("minecraft:lightning_rod").with("facing_direction=1,powered_bit=0"));
        ctx.set_weather(Weather::Thunder, None);
    });
    watch(&mut h, 200);
    let sent = draw(&mut h, 10, 10);
    let bolts: Vec<[f32; 3]> = added(&sent, "minecraft:lightning_bolt").iter().map(|bolt| [bolt.position.x, bolt.position.y, bolt.position.z]).collect();
    assert!(bolts == [[0.5, 66.0, 4.5]], "{bolts:?}");
    assert!(!all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "item.trident.thunder"));
}

/// bds-r1shots.txt: a crossbow held in use was heard loading 4, 14 and 24 ticks in, each sound no
/// one's, and was loaded at 25 with `ChargedItem`, before it was let go.
#[test]
fn a_crossbow_held_loads_by_itself_with_bds_s_sounds() {
    use acacia_proto::packets::{EntityEvent, EntityEventEventId};
    let mut h = ready(GameMode::Survival, &[("minecraft:crossbow", 1, 0), ARROWS], 0.0);
    h.receive(1, &common::shooting::click_air());
    let ticks: Vec<Vec<RawPacket>> = (0..30).map(|_| h.tick_all().to(1).to_vec()).collect();
    let when = |name: &str| ticks.iter().position(|sent| all::<LevelSoundEvent>(sent).iter().any(|sound| sound.sound_id == name && sound.entity_type.is_empty() && sound.entity_unique_id == -1));
    let charged = ticks.iter().position(|sent| all::<EntityEvent>(sent).iter().any(|event| event.event_id == EntityEventEventId::ChargedItem));
    assert_eq!([when("crossbow.loading.start"), when("crossbow.loading.middle"), when("crossbow.loading.end"), charged], [Some(4), Some(14), Some(24), Some(25)]);
}

#[test]
fn a_trident_with_loyalty_comes_back_to_the_hand() {
    let (mut h, _) = stored(vec![enchanted("minecraft:trident", &[(31, 3)], 0)], Vec::new());
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:cow", [0.5, 64.0, 3.5]).is_some()));
    face(&mut h, 1, PlayerId(1), [0.5, 64.0, 0.5], 5.0);
    let sent = draw(&mut h, 10, 60);
    assert_eq!(healths(&sent).first(), Some(&2.0));
    assert_eq!(all::<TakeItemEntity>(&sent).len(), 1, "caught");
    assert_eq!(held(&mut h, Slot::Main(0)).map(|trident| trident.name), Some("minecraft:trident".into()));
    let feet = common::asking::ask(&mut h, |ctx| ctx.position(PlayerId(1)).expect("Alex").feet);
    let back: Vec<[f32; 3]> = all::<LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id == "item.trident.return").map(|sound| [sound.position.x, sound.position.y, sound.position.z]).collect();
    assert_eq!(back, [[feet[0], feet[1] + sapling_data::EYE_HEIGHT, feet[2]]], "heard once at the eyes (bds-r1shots.txt)");
}
