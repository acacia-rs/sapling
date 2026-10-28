//! A player's breath and burning wait in storage (src/vitals/kept.rs).

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Metadata, MetadataDictionaryItemValueDefault as Plain};
use common::stored::enter_stored;
use common::{Harness, all, config};
use sapling_api::PlayerId;
use sapling_world::{MemoryStorage, World, WorldStorage};

#[test]
fn a_player_comes_back_short_of_breath_and_burning_and_is_kept_so() {
    let storage = Arc::new(MemoryStorage::default());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    let uuid = Arc::new(Mutex::new(String::new()));
    let steve = uuid.clone();
    h.remote.run(move |ctx| *steve.lock().unwrap() = ctx.players().next().unwrap().uuid.to_string());
    h.tick(1);
    h.leave(1);
    h.shard.tick();
    h.shard.save_worlds();
    let id = uuid.lock().unwrap().clone();
    let kept = storage.load_player(&id).unwrap().unwrap();
    assert_eq!((kept.air, kept.fire), (Some(300), Some(0)), "a fresh player is kept breathing and unlit");
    storage.store_player(&id, &sapling_world::PlayerData { air: Some(120), fire: Some(100), ..kept }).unwrap();

    let mut back = enter_stored(&mut h, 2);
    let breath: Vec<i16> = all::<SetEntityData>(&back)
        .iter()
        .flat_map(|data| data.metadata.iter().filter(|item| item.key == Key::Air).map(|item| item.value.clone()))
        .filter_map(|value| if let Metadata::Default(Plain::Short(air)) = value { Some(air) } else { None })
        .collect();
    // The join's own metadata says full breath first; the greeting corrects it.
    assert_eq!(breath.iter().find(|air| **air < 300), Some(&120), "told the breath it came back with: {breath:?}");
    (0..30).for_each(|_| back.extend(h.tick(2)));
    let health = Arc::new(Mutex::new(None));
    let seen = health.clone();
    h.remote.run(move |ctx| *seen.lock().unwrap() = ctx.health(PlayerId(2)));
    h.tick(2);
    assert!(health.lock().unwrap().is_some_and(|left| left < 20.0), "the fire it came back with burns it");
    h.leave(2);
    h.shard.tick();
    h.shard.save_worlds();
    let kept = storage.load_player(&id).unwrap().unwrap();
    assert!(kept.air.is_some_and(|air| air > 120) && kept.fire.is_some_and(|fire| (1..100).contains(&fire)), "{:?}", (kept.air, kept.fire));
}
