//! What a player's shots do to what they hit: a bow's enchantments, a crossbow's, and each thing
//! thrown (docs/research/projectiles.md, docs/research/items.md).

use crate::common;

use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::{RemoveEntity, SetEntityData, SetEntityMotion};
use acacia_proto::types::{MetadataDictionaryItemValue, MetadataFlags1};
use common::shooting::{ALEX, STEVE, added, draw, face, healths, held, ready, spawn, throw, watch};
use common::stored::{compound, enchanted, enter_stored, floor_with};
use common::{Harness, all, click, config};
use sapling_api::{GameMode, Item, PlayerId, Slot};
use sapling_world::{ItemStack, PlayerData, World, WorldStorage};

const COW: [f32; 3] = [0.5, 65.0, 3.5];
const POWER: i16 = 19;
const PUNCH: i16 = 20;
const FLAME: i16 = 21;
const QUICK_CHARGE: i16 = 35;
const PIERCING: i16 = 34;

/// Alex in survival with an arrow stack and a bow enchanted with `levels`, a cow before it.
fn archer(levels: &[(i16, i16)]) -> (Harness, u64) {
    let mut h = ready(GameMode::Survival, &[("minecraft:bow", 1, 0), ("minecraft:arrow", 8, 0)], 5.0);
    let bow = Item { enchantments: levels.to_vec(), ..Item::new("minecraft:bow", 1) };
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&bow))));
    h.tick_all();
    let cow = spawn(&mut h, "minecraft:cow", COW);
    (h, cow)
}

/// The fastest the mob `id` was told to move along +z.
fn thrown(sent: &[acacia_proto::RawPacket], id: u64) -> f32 {
    all::<SetEntityMotion>(sent).iter().filter(|motion| motion.runtime_entity_id == id).map(|motion| motion.velocity.z).fold(0.0, f32::max)
}

fn burning(sent: &[acacia_proto::RawPacket], id: u64) -> bool {
    let on_fire = |value: &MetadataDictionaryItemValue| matches!(value, MetadataDictionaryItemValue::Flags(flags) if flags.0 & MetadataFlags1::ONFIRE.0 != 0);
    all::<SetEntityData>(sent).iter().any(|data| data.runtime_entity_id == id && data.metadata.iter().any(|item| on_fire(&item.value)))
}

fn gone(sent: &[acacia_proto::RawPacket], kind: &str) -> bool {
    let shot = added(sent, kind).into_iter().next().expect("the shot");
    all::<RemoveEntity>(sent).iter().any(|removed| removed.entity_id_self == shot.unique_id)
}

#[test]
fn power_adds_half_a_level_to_a_bows_arrow() {
    let (mut h, _) = archer(&[(POWER, 1)]);
    // The half-drawn arrow that costs 3 plain (projectiles.rs) costs 1.5 times that, rounded up.
    assert_eq!(healths(&draw(&mut h, 10, 10)), [6.0]);
}

#[test]
fn punch_throws_the_mob_further_and_flame_sets_it_alight() {
    let (mut h, cow) = archer(&[]);
    let plain = draw(&mut h, 10, 4);
    assert!(!burning(&plain, cow));
    let (mut h, punched) = archer(&[(PUNCH, 1), (FLAME, 1)]);
    let sent = draw(&mut h, 10, 4);
    assert!(thrown(&sent, punched) > thrown(&plain, cow) + 0.5, "{} against {}", thrown(&sent, punched), thrown(&plain, cow));
    assert!(burning(&sent, punched), "Flame");
}

#[test]
fn a_bows_arrow_harms_a_survival_player_and_is_gone() {
    let mut h = ready(GameMode::Survival, &[("minecraft:bow", 1, 0), ("minecraft:arrow", 1, 0)], 5.0);
    h.remote.run(|ctx| ctx.set_game_mode(STEVE, GameMode::Survival));
    face(&mut h, 2, STEVE, COW, 0.0);
    let sent = draw(&mut h, 10, 10);
    let left = std::sync::Arc::new(std::sync::Mutex::new(None));
    let kept = left.clone();
    h.remote.run(move |ctx| *kept.lock().unwrap() = ctx.health(STEVE));
    h.tick_all();
    assert_eq!(*left.lock().unwrap(), Some(17.0));
    assert!(gone(&sent, "minecraft:arrow"), "an arrow in a player is not stuck");
}

/// Alex with `main` in a stored world, survival, standing on its floor looking 5 down along +z.
fn stored(main: Vec<Option<ItemStack>>) -> Harness {
    let storage = floor_with(Vec::new());
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &PlayerData { main, ..PlayerData::default() }).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage));
    enter_stored(&mut h, 1);
    (0..12).for_each(|_| drop(h.tick(1)));
    h.remote.run(|ctx| ctx.set_game_mode(PlayerId(1), GameMode::Survival));
    face(&mut h, 1, PlayerId(1), [0.5, 64.0, 0.5], 5.0);
    h
}

fn arrows(count: u8) -> Option<ItemStack> {
    Some(ItemStack { name: "minecraft:arrow".into(), count, damage: 0, block: None, tag: None })
}

#[test]
fn quick_charge_three_loads_in_ten_ticks() {
    let mut h = stored(vec![enchanted("minecraft:crossbow", &[(QUICK_CHARGE, 3)], 0), arrows(2)]);
    draw(&mut h, 9, 1);
    assert_eq!(held(&mut h, Slot::Main(1)).map(|arrows| arrows.count), Some(2), "let go too soon");
    draw(&mut h, 10, 1);
    assert_eq!(held(&mut h, Slot::Main(1)).map(|arrows| arrows.count), Some(1), "loaded");
}

#[test]
fn a_piercing_arrow_harms_the_cow_behind_the_first() {
    let mut h = stored(vec![enchanted("minecraft:crossbow", &[(PIERCING, 1)], 0), arrows(1)]);
    draw(&mut h, 25, 1);
    spawn(&mut h, "minecraft:cow", [0.5, 64.0, 3.5]);
    spawn(&mut h, "minecraft:cow", [0.5, 64.0, 6.5]);
    let sent = throw(&mut h, 10);
    assert_eq!(healths(&sent).len(), 2, "{:?}", healths(&sent));
}

#[test]
fn a_snowball_costs_a_blaze_three_and_an_egg_harms_a_cow_nothing() {
    let mut h = ready(GameMode::Creative, &[("minecraft:snowball", 1, 0)], 5.0);
    h.remote.run(|ctx| ctx.set_difficulty(sapling_api::Difficulty::Easy));
    spawn(&mut h, "minecraft:blaze", [0.5, 65.0, 3.5]);
    let sent = throw(&mut h, 10);
    assert_eq!(healths(&sent).first(), Some(&17.0));
    assert!(gone(&sent, "minecraft:snowball"));

    let mut h = ready(GameMode::Creative, &[("minecraft:egg", 1, 0)], 5.0);
    let cow = spawn(&mut h, "minecraft:cow", COW);
    let sent = throw(&mut h, 10);
    assert!(healths(&sent).iter().all(|left| *left == 10.0), "{:?}", healths(&sent));
    assert!(thrown(&sent, cow) > 0.0, "thrown back");
    assert!(gone(&sent, "minecraft:egg"));
}

#[test]
fn a_lingering_potion_of_harming_harms_a_cow_standing_in_its_cloud() {
    let mut h = ready(GameMode::Creative, &[("minecraft:lingering_potion", 1, 23)], 30.0);
    spawn(&mut h, "minecraft:cow", [0.5, 65.0, 2.5]);
    let sent = throw(&mut h, 40);
    assert!(gone(&sent, "minecraft:lingering_potion"));
    // Half an instant harm, then spared for 40 ticks (AreaEffectCloud::normalTick 1429b01a0).
    assert_eq!(healths(&sent), [7.0]);
}

#[test]
fn a_rocket_with_a_star_harms_a_cow_where_it_bursts() {
    let burst = compound([("FireworkColor", Value::ByteArray(vec![1])), ("FireworkFade", Value::ByteArray(Vec::new())), ("FireworkFlicker", Value::Byte(0)), ("FireworkTrail", Value::Byte(0)), ("FireworkType", Value::Byte(0))]);
    let fireworks = compound([("Explosions", Value::List(List { tag: 10, items: vec![burst] })), ("Flight", Value::Byte(0))]);
    let rocket = ItemStack { name: "minecraft:firework_rocket".into(), count: 1, damage: 0, block: None, tag: Some(compound([("Fireworks", fireworks)])) };
    let mut h = stored(vec![Some(rocket)]);
    spawn(&mut h, "minecraft:cow", [0.5, 64.0, 3.5]);
    h.receive(1, &click([0, 63, 2], 1, None));
    let sent = watch(&mut h, 30);
    assert!(gone(&sent, "minecraft:fireworks_rocket"));
    assert!(healths(&sent).iter().any(|left| *left < 10.0), "{:?}", healths(&sent));
}
