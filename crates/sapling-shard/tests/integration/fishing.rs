//! Fishing: a rod casts a hook and reels it in; a fish bites in water and gives a catch, experience
//! and wear; a hooked mob is pulled (docs/research/fishing.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value as Nbt;
use acacia_proto::packets::{AddItemEntity, CompletedUsingItem, EntityEvent, EntityEventEventId, InventoryContent, RemoveEntity, SetEntityData};
use acacia_proto::types::{ItemV4Extra, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, WindowIDVarint};
use common::orbs::worths;
use common::shooting::{ALEX, added, ready, spawn, throw, watch};
use common::{Harness, all};
use sapling_api::{Block, GameMode, Item, Slot};

const ROD: &str = "minecraft:fishing_rod";
const HOOK: &str = "minecraft:fishing_hook";
const LURE: i16 = 24;

/// Alex with a rod (Lure `lure`, if any) looking `pitch` down along +z over a pool of water.
fn at_the_pool(lure: i16, pitch: f32) -> Harness {
    let mut h = ready(GameMode::Survival, &[], pitch);
    h.remote.run(move |ctx| {
        let enchantments = if lure > 0 { vec![(LURE, lure)] } else { Vec::new() };
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item { enchantments, ..Item::new(ROD, 1) })));
        for x in -3..=3 {
            for z in 2..=20 {
                assert!(ctx.set_block([x, 63, z], &Block::new("stone")));
                assert!(ctx.set_block([x, 64, z], &Block::new("water")));
            }
        }
    });
    h.tick_all();
    h
}

/// The rod's wear as the last inventory sent in `sent` shows it.
fn wear(sent: &[RawPacket]) -> Option<i32> {
    let windows = all::<InventoryContent>(sent);
    let inventory = windows.iter().rev().find(|window| window.window_id == WindowIDVarint::Inventory)?;
    let ItemV4Extra::Default(Some(rod)) = &inventory.input[0].extra else { return None };
    match rod.nbt.as_ref().and_then(|tag| tag.nbt.value.get("Damage")) {
        Some(Nbt::Int(worn)) => Some(*worn),
        _ => Some(0),
    }
}

#[test]
fn a_cast_spawns_a_hook_its_fisher_owns_and_a_use_reels_it_back_unworn() {
    let mut h = at_the_pool(0, 20.0);
    // Reeled while it flies: a hook against a block wears the rod by 2.
    let sent = throw(&mut h, 0);
    let hook = added(&sent, HOOK);
    assert_eq!(hook.len(), 1);
    let owner = hook[0].metadata.iter().find(|item| item.key == Key::OwnerEid).map(|item| item.value.clone());
    assert!(matches!(owner, Some(Value::Default(Plain::Long(id))) if id != 0), "{owner:?}");
    let speed = [hook[0].velocity.x, hook[0].velocity.y, hook[0].velocity.z].iter().map(|c| c * c).sum::<f32>().sqrt();
    assert!((speed - 1.3).abs() < 1e-3, "BDS casts at 1.3: {speed}");
    assert_eq!(all::<CompletedUsingItem>(&sent).len(), 1);
    let sent = throw(&mut h, 2);
    assert_eq!(all::<RemoveEntity>(&sent).len(), 1, "reeled in");
    assert!(added(&sent, HOOK).is_empty());
    assert_eq!(wear(&sent), Some(0), "nothing on the line");
}

#[test]
fn a_fish_bites_in_water_and_reeling_it_gives_a_catch_experience_and_one_wear() {
    let mut h = at_the_pool(3, 20.0);
    let mut seen = throw(&mut h, 1);
    let bit = (0..900).any(|_| {
        let sent = watch(&mut h, 1);
        seen.extend(sent.clone());
        all::<EntityEvent>(&sent).iter().any(|event| event.event_id == EntityEventEventId::FishHookHook)
    });
    let moves: Vec<_> = all::<acacia_proto::packets::MoveEntityDelta>(&seen).into_iter().take(12).map(|delta| (delta.y, delta.z, delta.on_ground)).collect();
    assert!(bit, "a bite within 45 seconds with Lure III: {:?} {moves:?}", added(&seen, HOOK).first().map(|hook| hook.position.clone()));
    let sent = throw(&mut h, 2);
    let caught = all::<AddItemEntity>(&sent);
    assert_eq!(caught.len(), 1, "one catch");
    assert!(caught[0].is_from_fishing);
    assert!((caught[0].velocity.z) < 0.0, "pulled back toward the fisher");
    assert!(matches!(worths(&sent).iter().sum::<i32>(), 1..=5));
    assert_eq!(wear(&sent), Some(1));
}

#[test]
fn a_hooked_cow_is_held_and_pulled_for_three_wear() {
    let mut h = at_the_pool(0, 0.0);
    let cow = spawn(&mut h, "minecraft:cow", [0.5, 65.0, 1.8]);
    let sent = throw(&mut h, 5);
    let target = all::<SetEntityData>(&sent).into_iter().flat_map(|data| data.metadata).find(|item| item.key == Key::TargetEid).map(|item| item.value);
    assert!(matches!(target, Some(Value::Default(Plain::Long(id))) if id as u64 == cow), "{target:?}");
    let sent = throw(&mut h, 2);
    assert_eq!(wear(&sent), Some(3));
}
