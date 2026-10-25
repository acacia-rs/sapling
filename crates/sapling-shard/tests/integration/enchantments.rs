//! Enchantments at work (docs/research/enchantments.md): a player comes back from storage with
//! enchanted stacks, which plugins cannot make.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::{AddEntity, AddItemEntity, UpdateAttributes};
use common::stored::{compound, enter_stored, floor_with};
use common::{Harness, all, attack, config};
use sapling_api::{DamageCause, PlayerId};
use sapling_world::{ItemStack, MemoryStorage, PlayerData, World, WorldStorage, new_orb_actor};

const STEVE: PlayerId = PlayerId(1);

/// A stack of one `name` with `levels` of enchantments (by id) and worn by `damage`.
fn enchanted(name: &str, levels: &[(i16, i16)], damage: i32) -> Option<ItemStack> {
    let entry = |(id, level): &(i16, i16)| compound([("id", Value::Short(*id)), ("lvl", Value::Short(*level))]);
    let tag = compound([("ench", Value::List(List { tag: 10, items: levels.iter().map(entry).collect() })), ("Damage", Value::Int(damage))]);
    Some(ItemStack { name: name.into(), count: 1, damage: 0, block: None, tag: Some(tag) })
}

fn plain(name: &str) -> Option<ItemStack> {
    Some(ItemStack { name: name.into(), count: 1, damage: 0, block: None, tag: None })
}

/// Steve, as `data` kept him, in a world whose column holds `actors`.
fn steve(data: PlayerData, actors: Vec<u8>) -> (Harness, Arc<MemoryStorage>) {
    let storage = floor_with(actors);
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &data).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    (0..12).for_each(|_| drop(h.tick(1)));
    (h, storage)
}

fn health(sent: &[acacia_proto::RawPacket]) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect()
}

#[test]
fn sharpness_adds_its_bonus_to_a_hit_on_a_mob() {
    let data = PlayerData { main: vec![enchanted("minecraft:wooden_sword", &[(9, 2)], 0)], ..PlayerData::default() };
    let (mut h, _) = steve(data, Vec::new());
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:cow", [2.5, 64.0, 0.5]).is_some()));
    let cow = (0..5).flat_map(|_| all::<AddEntity>(&h.tick(1))).find(|added| added.entity_type == "minecraft:cow").expect("the cow").runtime_id;
    h.receive(1, &attack(cow));
    // A wooden sword's 5 and Sharpness II's 2.5, floored, of the cow's 10.
    assert_eq!(health(&h.tick(1)), [3.0]);
}

#[test]
fn protection_takes_its_share_off_a_hit_behind_the_armour() {
    let data = PlayerData { armor: vec![None, None, None, enchanted("minecraft:leather_boots", &[(0, 4), (2, 4)], 0)], ..PlayerData::default() };
    let (mut h, _) = steve(data, Vec::new());
    let left = Arc::new(Mutex::new(Vec::new()));
    let seen = left.clone();
    h.remote.run(move |ctx| {
        assert!(ctx.hurt_player(STEVE, 10.0, DamageCause::Generic));
        seen.lock().unwrap().push(ctx.health(STEVE).unwrap());
    });
    h.tick(1);
    // The boots' point lets 9.92 through, Protection IV's 4 points 84 hundredths of that.
    let taken = 20.0 - left.lock().unwrap()[0];
    assert!((taken - 9.92 * 0.84).abs() < 1e-3, "{taken}");
}

#[test]
fn an_orb_mends_a_worn_piece_with_mending_before_it_gives_experience() {
    let data = PlayerData { armor: vec![None, None, None, enchanted("minecraft:iron_boots", &[(26, 1)], 30)], ..PlayerData::default() };
    let (mut h, storage) = steve(data, new_orb_actor(-5, 10, [0.5, 64.0, 0.5], [0.0; 3]));
    let told: Vec<_> = (0..40).flat_map(|_| h.tick(1)).collect();
    let progress = all::<UpdateAttributes>(&told).iter().flat_map(|update| update.attributes.clone()).filter(|attribute| attribute.name == "minecraft:player.experience").count();
    assert_eq!(progress, 0, "all ten went into the boots");
    h.leave(1);
    h.shard.tick();
    h.shard.save_worlds();
    let kept = storage.load_player(&acacia_auth::login::offline_identity("Steve")).unwrap().unwrap();
    let worn = kept.armor[3].as_ref().and_then(|boots| boots.tag.as_ref()?.get("Damage").cloned());
    assert_eq!(worn, Some(Value::Int(10)), "two of wear a point");
}

#[test]
fn a_piece_with_curse_of_binding_stays_on() {
    use acacia_proto::types::{ContainerSlotType as Container, ItemStackResponsesItemStatus as Status};
    use common::requests::{answer, request, slot, take};
    // Ids go out in slot order: the helmet 1, the boots 2.
    let data = PlayerData { armor: vec![enchanted("minecraft:iron_helmet", &[(27, 1)], 0), None, None, plain("minecraft:iron_boots")], ..PlayerData::default() };
    let (mut h, _) = steve(data, Vec::new());
    h.receive(1, &request(-1, vec![take(1, slot(Container::Armor, 0, 1), slot(Container::Cursor, 0, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Error);
    h.receive(1, &request(-3, vec![take(1, slot(Container::Armor, 3, 2), slot(Container::Cursor, 0, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
}

#[test]
fn a_stack_with_curse_of_vanishing_is_gone_at_death() {
    let data = PlayerData { main: vec![plain("minecraft:stone"), enchanted("minecraft:diamond_sword", &[(28, 1)], 0)], ..PlayerData::default() };
    let (mut h, _) = steve(data, Vec::new());
    h.remote.run(|ctx| assert!(ctx.kill_player(STEVE)));
    let dropped: Vec<_> = (0..3).flat_map(|_| all::<AddItemEntity>(&h.tick(1))).collect();
    assert_eq!(dropped.len(), 1, "the stone and not the sword");
}
