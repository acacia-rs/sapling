//! What eating does besides feeding, what an empty hunger does and how a `/damage` death is said,
//! against research/captures/bds-foods2.txt, bds-fx3.txt, bds-hurt2.txt and bds-deaths.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{CompletedUsingItem, DeathInfo, LevelSoundEvent, MobEffect, MovePlayer, MovePlayerTeleportCause, Respawn, UpdateAttributes};
use acacia_proto::types::{TransactionTransactionData, TransactionUseItemActionType, Vec3f};
use common::{Harness, all};
use sapling_api::{DamageCause, Difficulty, Effect, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

/// Alex, hungry, eats `item` from its hand; what it and Steve were sent over 50 ticks.
fn swallow(h: &mut Harness, item: Item) -> (Vec<RawPacket>, Vec<RawPacket>) {
    let mut click = common::click([0, 0, 0], 255, None);
    if let TransactionTransactionData::ItemUse(used) = &mut click.transaction.transaction_data {
        used.action_type = TransactionUseItemActionType::ClickAir;
    }
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::Hunger, 60, 255, false)));
    (0..62).for_each(|_| drop(h.tick_all()));
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&item))));
    h.tick_all();
    h.receive(1, &click);
    let (mut alex, mut steve) = (Vec::new(), Vec::new());
    for _ in 0..50 {
        let sent = h.tick_all();
        alex.extend_from_slice(sent.to(1));
        steve.extend_from_slice(sent.to(2));
    }
    (alex, steve)
}

fn sounds(sent: &[RawPacket]) -> Vec<String> {
    all::<LevelSoundEvent>(sent).into_iter().map(|sound| sound.sound_id).filter(|id| id != "burp").collect()
}

#[test]
fn a_chorus_fruit_sends_its_eater_off_and_a_stew_gives_its_flower() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let (alex, _) = swallow(&mut h, Item { damage: 9, ..Item::new("minecraft:suspicious_stew", 1) });
    let given: Vec<(i32, i32)> = all::<MobEffect>(&alex).into_iter().filter(|effect| effect.effect_id != 17).map(|effect| (effect.effect_id, effect.duration)).collect();
    assert_eq!(given, [(20, 140), (20, 140)], "wither for 7 seconds, added then updated");
    assert!(!sounds(&alex).iter().any(|id| id == "pop"), "the bowl is left in the hand unheard");
    h.remote.run(|ctx| assert_eq!(ctx.item(ALEX, Slot::Main(0)).map(|item| item.name), Some("minecraft:bowl".to_owned())));
    h.tick_all();
    let (alex, steve) = swallow(&mut h, Item::new("minecraft:chorus_fruit", 1));
    let moved: Vec<MovePlayer> = all::<MovePlayer>(&alex).into_iter().filter(|moved| moved.teleport.as_ref().is_some_and(|told| told.cause == MovePlayerTeleportCause::ChorusFruit)).collect();
    assert_eq!(moved.len(), 1, "one teleport, told as the fruit's");
    assert_eq!((sounds(&steve), all::<CompletedUsingItem>(&alex).len()), (vec!["teleport".to_owned(), "teleport".to_owned()], 1));
    h.tick_all();
}

#[test]
fn a_saturation_stew_feeds_hunger_each_of_its_six_ticks_and_leaves_saturation_be() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let (alex, _) = swallow(&mut h, Item { damage: 5, ..Item::new("minecraft:suspicious_stew", 1) });
    // The meal's update and the effect's six after it: hunger a point a tick, saturation as the meal left it.
    let told = |name: &str| all::<UpdateAttributes>(&alex).into_iter().flat_map(|update| update.attributes).filter(|told| told.name == name).map(|told| told.current).collect::<Vec<_>>();
    let (hunger, saturation) = (told("minecraft:player.hunger"), told("minecraft:player.saturation"));
    let fed = &hunger[hunger.len().saturating_sub(7)..];
    assert!(fed.len() == 7 && fed.windows(2).all(|pair| pair[1] - pair[0] == 1.0), "{hunger:?}");
    assert!(saturation[saturation.len().saturating_sub(7)..].windows(2).all(|pair| pair[0] == pair[1]), "{saturation:?}");
    let given: Vec<i32> = all::<MobEffect>(&alex).into_iter().filter(|effect| effect.effect_id == 23).map(|effect| effect.duration).collect();
    assert_eq!(given, [6, 6, 0], "added, updated, removed");
}

/// Alex's health after 3 left and 30 seconds of a strong hunger on `difficulty`.
fn starved_to(difficulty: Difficulty) -> Option<f32> {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(difficulty);
        assert!(ctx.hurt_player(ALEX, 17.0, DamageCause::Magic) && ctx.give_effect(ALEX, Effect::Hunger, 600, 255, false));
    });
    (0..600).for_each(|_| drop(h.tick_all()));
    common::asking::ask(&mut h, |ctx| ctx.health(ALEX))
}

#[test]
fn an_empty_hunger_stops_at_ten_on_easy_one_on_normal_and_kills_on_hard() {
    assert_eq!([Difficulty::Easy, Difficulty::Normal].map(starved_to), [Some(3.0), Some(1.0)]);
    assert!(starved_to(Difficulty::Hard).is_none_or(|left| left <= 0.0), "starved to death");
}

#[test]
fn a_told_death_is_said_as_bds_said_damage_of_its_cause() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let mut said = Vec::new();
    for cause in [DamageCause::Fall, DamageCause::Contact, DamageCause::BlockExplosion, DamageCause::Lava] {
        h.remote.run(move |ctx| assert!(ctx.hurt_player(ALEX, 100.0, cause)));
        said.extend((0..3).flat_map(|_| all::<DeathInfo>(h.tick_all().to(1))).map(|info| info.cause));
        h.receive(1, &Respawn { position: Vec3f { x: 0.0, y: 0.0, z: 0.0 }, state: 2, runtime_entity_id: 0 });
        (0..30).for_each(|_| drop(h.tick_all()));
    }
    assert_eq!(said, ["death.attack.fall", "death.attack.generic", "death.attack.explosion.by.bed", "death.attack.lava"]);
}

#[test]
fn an_ominous_bottle_gives_its_omen_unseen_and_is_heard_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let (alex, _) = swallow(&mut h, Item { damage: 2, ..Item::new("minecraft:ominous_bottle", 1) });
    let omen: Vec<(i32, bool)> = all::<MobEffect>(&alex).into_iter().filter(|effect| effect.effect_id == 28).map(|effect| (effect.amplifier, effect.particles)).collect();
    assert_eq!(omen, [(2, false)]);
    assert_eq!(sounds(&alex), ["apply_effect.bad_omen", "ominous_bottle.end_use"]);
    h.tick_all();
}
