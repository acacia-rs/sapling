//! A player's bow, crossbow and trident (docs/research/projectiles.md).

use crate::common;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{LevelSoundEvent, TakeItemEntity};
use acacia_proto::types::{MetadataDictionaryItemValue, MetadataFlags1};
use common::shooting::{ALEX, added, draw, face, healths, held, ready, spawn, throw, watch};
use common::stored::{actor, enchanted, enter_stored, floor_with, plain};
use common::{Harness, all, config};
use sapling_api::{GameMode, Item, PlayerId, Slot};
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

#[test]
fn a_trident_with_loyalty_comes_back_to_the_hand() {
    let (mut h, _) = stored(vec![enchanted("minecraft:trident", &[(31, 3)], 0)], Vec::new());
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:cow", [0.5, 64.0, 3.5]).is_some()));
    face(&mut h, 1, PlayerId(1), [0.5, 64.0, 0.5], 5.0);
    let sent = draw(&mut h, 10, 60);
    assert_eq!(healths(&sent).first(), Some(&2.0));
    assert_eq!(all::<TakeItemEntity>(&sent).len(), 1, "caught");
    assert_eq!(held(&mut h, Slot::Main(0)).map(|trident| trident.name), Some("minecraft:trident".into()));
}
