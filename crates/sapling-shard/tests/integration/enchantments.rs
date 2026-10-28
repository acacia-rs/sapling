//! Enchantments at work (docs/research/enchantments.md): a player comes back from storage with
//! enchanted stacks, which plugins cannot make.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddEntity, AddItemEntity, UpdateAttributes};
use common::stored::{enchanted, enter_stored, floor_with, plain};
use common::{Harness, all, attack, config};
use sapling_api::{Block, DamageCause, Item, PlayerId, Slot};
use sapling_world::{MemoryStorage, PlayerData, World, WorldStorage, new_orb_actor};

const STEVE: PlayerId = PlayerId(1);

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

fn alex_wearing(slot: u8, name: &str, enchantments: Vec<(i16, i16)>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let worn = Item { enchantments, ..Item::new(name, 1) };
    h.remote.run(move |ctx| assert!(ctx.set_item(PlayerId(1), Slot::Armor(slot), Some(&worn))));
    h.tick_all();
    h
}

fn walking(sent: &[acacia_proto::RawPacket]) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:movement").map(|attribute| attribute.current).collect()
}

#[test]
fn soul_speed_quickens_its_wearer_each_tick_on_soul_sand_and_lets_go_off_it() {
    // research/captures/bds-soles.txt: 0.1615 retold each tick in Soul Speed III, 0.1 once off.
    let mut h = alex_wearing(3, "minecraft:diamond_boots", vec![(36, 3)]);
    let under = Arc::new(Mutex::new([0; 3]));
    let found = under.clone();
    h.remote.run(move |ctx| {
        let [x, y, z] = ctx.position(PlayerId(1)).unwrap().feet;
        let cell = [x.floor() as i32, y.floor() as i32 - 1, z.floor() as i32];
        *found.lock().unwrap() = cell;
        assert!(ctx.set_block(cell, &Block::new("soul_sand")));
    });
    let told: Vec<_> = (0..3).flat_map(|_| walking(h.tick_all().to(1))).collect();
    assert!(told.len() >= 2 && told.iter().all(|speed| (speed - 0.1615).abs() < 1e-6), "{told:?}");
    let cell = *under.lock().unwrap();
    h.remote.run(move |ctx| assert!(ctx.set_block(cell, &Block::new("stone"))));
    // The tick the block is set in still told the quickened speed.
    let told: Vec<_> = (0..3).flat_map(|_| walking(h.tick_all().to(1))).collect();
    assert_eq!(told, [0.1615, 0.1]);
}

#[test]
fn eyes_in_water_crack_a_block_five_times_slower_unless_a_helmet_has_aqua_affinity() {
    use acacia_proto::packets::{LevelEvent, LevelEventEvent};
    use acacia_proto::types::Action;
    // research/captures/bds-soles.txt: dirt by hand cracked at 873 under water, 4369 with Aqua Affinity.
    for (enchantments, ticks) in [(vec![], 75), (vec![(8, 1)], 15)] {
        let mut h = alex_wearing(0, "minecraft:iron_helmet", enchantments);
        h.remote.run(|ctx| assert!([[0, 65, 0], [0, 66, 0]].iter().all(|at| ctx.set_block(*at, &Block::new("water"))) && ctx.set_block([1, 64, 0], &Block::new("dirt"))));
        h.tick_all();
        h.receive(1, &common::block_actions(&[(Action::StartBreak, [1, 64, 0])]));
        let cracks: Vec<_> = all::<LevelEvent>(h.tick_all().to(2)).into_iter().filter(|event| event.event == LevelEventEvent::BlockStartBreak).map(|event| event.data).collect();
        assert_eq!(cracks, [65535 / ticks]);
    }
}

#[test]
fn a_stack_with_curse_of_vanishing_is_gone_at_death() {
    let data = PlayerData { main: vec![plain("minecraft:stone"), enchanted("minecraft:diamond_sword", &[(28, 1)], 0)], ..PlayerData::default() };
    let (mut h, _) = steve(data, Vec::new());
    h.remote.run(|ctx| assert!(ctx.kill_player(STEVE)));
    let dropped: Vec<_> = (0..3).flat_map(|_| all::<AddItemEntity>(&h.tick(1))).collect();
    assert_eq!(dropped.len(), 1, "the stone and not the sword");
}
