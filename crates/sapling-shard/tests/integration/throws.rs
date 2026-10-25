//! Things a player throws: ender pearls, snowballs, eggs, potions, bottles o' enchanting and wind
//! charges (docs/research/projectiles.md).

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::{LevelSoundEvent, RemoveEntity};
use common::orbs::worths;
use common::shooting::{ALEX, FEET, added, healths, held, ready, spawn, throw};
use common::{Harness, all};
use sapling_api::{GameMode, Slot};

fn count(h: &mut Harness) -> Option<u8> {
    held(h, Slot::Main(0)).map(|stack| stack.count)
}

#[test]
fn an_ender_pearl_takes_its_thrower_where_it_lands_for_five_and_cools_down() {
    let mut h = ready(GameMode::Survival, &[("minecraft:ender_pearl", 3, 0)], 45.0);
    let sent = throw(&mut h, 2);
    assert_eq!(added(&sent, "minecraft:ender_pearl").len(), 1);
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "throw"));
    assert!(added(&throw(&mut h, 30), "minecraft:ender_pearl").is_empty(), "cooling down");
    assert_eq!(count(&mut h), Some(2));
    let seen = Arc::new(Mutex::new(None));
    let kept = seen.clone();
    h.remote.run(move |ctx| *kept.lock().unwrap() = Some((ctx.position(ALEX).unwrap().feet, ctx.health(ALEX).unwrap())));
    h.tick_all();
    let (feet, health) = seen.lock().unwrap().take().unwrap();
    assert!(feet[2] > FEET[2] + 1.0 && (feet[1] - 65.0).abs() < 0.5, "where it landed: {feet:?}");
    assert_eq!(health, 15.0);
}

#[test]
fn a_snowball_is_spent_and_breaks_where_it_lands_and_a_creative_egg_is_not_spent() {
    let mut h = ready(GameMode::Survival, &[("minecraft:snowball", 2, 0)], 30.0);
    let sent = throw(&mut h, 20);
    let ball = added(&sent, "minecraft:snowball");
    assert_eq!(ball.len(), 1);
    assert!(all::<RemoveEntity>(&sent).iter().any(|gone| gone.entity_id_self == ball[0].unique_id), "it broke");
    assert_eq!(count(&mut h), Some(1));

    let mut h = ready(GameMode::Creative, &[("minecraft:egg", 2, 0)], 30.0);
    assert_eq!(added(&throw(&mut h, 20), "minecraft:egg").len(), 1);
    assert_eq!(count(&mut h), Some(2));
}

#[test]
fn a_bottle_o_enchanting_breaks_into_three_to_ten_experience() {
    let mut h = ready(GameMode::Survival, &[("minecraft:experience_bottle", 1, 0)], 60.0);
    let sent = throw(&mut h, 30);
    assert_eq!(added(&sent, "minecraft:xp_bottle").len(), 1);
    let worth: i32 = worths(&sent).iter().sum();
    assert!((3..=10).contains(&worth), "{worth}");
    assert_eq!(count(&mut h), None);
}

#[test]
fn a_splash_potion_of_harming_harms_a_cow_and_a_lingering_one_leaves_a_cloud() {
    let mut h = ready(GameMode::Creative, &[("minecraft:splash_potion", 1, 23)], 30.0);
    spawn(&mut h, "minecraft:cow", [0.5, 65.0, 2.5]);
    let sent = throw(&mut h, 20);
    assert_eq!(added(&sent, "minecraft:splash_potion").len(), 1);
    assert!(healths(&sent).iter().any(|left| *left < 10.0), "{:?}", healths(&sent));

    let mut h = ready(GameMode::Creative, &[("minecraft:lingering_potion", 1, 23)], 30.0);
    let sent = throw(&mut h, 20);
    assert_eq!((added(&sent, "minecraft:lingering_potion").len(), added(&sent, "minecraft:area_effect_cloud").len()), (1, 1));
}

#[test]
fn a_wind_charge_is_spent_and_bursts() {
    let mut h = ready(GameMode::Survival, &[("minecraft:wind_charge", 2, 0)], 60.0);
    let sent = throw(&mut h, 10);
    assert_eq!(added(&sent, "minecraft:wind_charge_projectile").len(), 1);
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "wind_charge.burst"));
    assert_eq!(count(&mut h), Some(1));
}
