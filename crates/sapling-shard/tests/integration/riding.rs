//! Riding: seats taken by a click and by the API, left on `LeaveVehicle`, told to viewers with
//! `SetEntityLink` (research/captures/bds-mounts.txt), saddles, and links kept in a world folder.

use std::sync::{Arc, Mutex};

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::{Interact, InteractActionId, InventoryTransaction, SetEntityLink};
use acacia_proto::types::{TransactionTransactionData, TransactionTransactionDataItemUseOnEntityActionType as OnEntity};
use common::stored::{actor, compound, enter_stored, floor_with};
use common::{Harness, all, attack, config};
use sapling_api::{EntityId, Item, Mount, PlayerId, Slot, Target};
use sapling_world::World;

const ALEX: PlayerId = PlayerId(1);

/// A click (not a hit) by the player at the platform's spawn on the entity `id`.
pub fn interact(id: u64) -> InventoryTransaction {
    let mut used = attack(id);
    if let TransactionTransactionData::ItemUseOnEntity(on) = &mut used.transaction.transaction_data {
        on.action_type = OnEntity::Interact;
    }
    used
}

/// Spawns `kinds` at `feet` and returns their ids, in order.
pub fn spawned(h: &mut Harness, kinds: &'static [(&'static str, [f32; 3])]) -> Vec<EntityId> {
    let ids = Arc::new(Mutex::new(Vec::new()));
    let slot = ids.clone();
    h.remote.run(move |ctx| *slot.lock().unwrap() = kinds.iter().map(|(kind, feet)| ctx.spawn_entity(kind, *feet).expect(kind)).collect());
    h.tick(1);
    let ids = ids.lock().unwrap().clone();
    ids
}

/// The (ridden, rider, type) of each link told.
pub fn links(sent: &[RawPacket]) -> Vec<(i64, i64, u8)> {
    all::<SetEntityLink>(sent).into_iter().map(|told| (told.link.ridden_entity_id, told.link.rider_entity_id, told.link.r#type)).collect()
}

fn ticks(h: &mut Harness, count: usize) -> Vec<RawPacket> {
    (0..count).flat_map(|_| h.tick(1)).collect()
}

#[test]
fn a_click_on_a_boat_seats_the_player_and_leave_vehicle_lets_it_off() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let boat = spawned(&mut h, &[("minecraft:boat", [2.5, 65.0, 0.5])])[0];
    h.receive(1, &interact(boat.0));
    let sent = ticks(&mut h, 2);
    assert!(links(&sent).contains(&(boat.0 as i64, 1, 1)), "the player drives: {:?}", links(&sent));
    h.remote.run(move |ctx| {
        let alex = ctx.players().find(|player| player.id == ALEX).expect("Alex");
        assert_eq!(ctx.riders(boat), vec![Target::Player(alex.clone())]);
        assert_eq!(ctx.vehicle_of(&Target::Player(alex)), Some(boat));
        assert_eq!(ctx.seats_left(boat), 1);
    });
    h.tick(1);
    h.receive(1, &Interact { action_id: InteractActionId::LeaveVehicle, target_entity_id: boat.0, has_position: false, position: None });
    let sent = ticks(&mut h, 2);
    assert!(links(&sent).contains(&(boat.0 as i64, 1, 0)), "{:?}", links(&sent));
    h.remote.run(move |ctx| assert!(ctx.riders(boat).is_empty()));
    h.tick(1);
}

#[test]
fn the_api_seats_mobs_until_a_boat_is_full_and_eviction_empties_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let ids = spawned(&mut h, &[("minecraft:boat", [2.5, 65.0, 0.5]), ("minecraft:pig", [4.5, 65.0, 0.5]), ("minecraft:pig", [4.5, 65.0, 2.5]), ("minecraft:pig", [4.5, 65.0, -2.5])]);
    let (boat, pigs) = (ids[0], ids[1..].to_vec());
    h.remote.run(move |ctx| {
        let seated: Vec<bool> = pigs.iter().map(|pig| ctx.ride(&Target::Entity(*pig), boat, Mount::RiderToVehicle)).collect();
        assert_eq!(seated, [true, true, false], "a boat has two seats");
        assert_eq!(ctx.riders(boat), vec![Target::Entity(pigs[0]), Target::Entity(pigs[1])]);
        assert!(!ctx.ride(&Target::Entity(boat), pigs[2], Mount::RiderToVehicle), "an unsaddled pig takes no rider");
    });
    let sent = ticks(&mut h, 2);
    assert_eq!(links(&sent).iter().filter(|(ridden, ..)| *ridden == boat.0 as i64).count(), 2, "{:?}", links(&sent));
    h.remote.run(move |ctx| {
        assert_eq!(ctx.evict_riders(boat), 2);
        assert_eq!(ctx.seats_left(boat), 2);
    });
    let sent = ticks(&mut h, 2);
    assert_eq!(links(&sent).iter().filter(|(_, _, kind)| *kind == 0).count(), 2);
}

#[test]
fn a_saddle_put_on_a_pig_is_spent_and_the_next_click_rides_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let pig = spawned(&mut h, &[("minecraft:pig", [2.5, 65.0, 0.5])])[0];
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:saddle", 1)))));
    h.tick(1);
    h.receive(1, &interact(pig.0));
    ticks(&mut h, 2);
    h.remote.run(move |ctx| {
        assert!(ctx.item(ALEX, Slot::Main(0)).is_none(), "the saddle went on the pig");
        assert!(ctx.riders(pig).is_empty(), "saddling is all the first click does");
    });
    h.tick(1);
    h.receive(1, &interact(pig.0));
    let sent = ticks(&mut h, 2);
    assert!(links(&sent).contains(&(pig.0 as i64, 1, 1)), "{:?}", links(&sent));
}

#[test]
fn a_boat_kept_with_its_rider_takes_it_back_when_loaded() {
    let link = compound([("entityID", Value::Long(-20)), ("linkID", Value::Int(0))]);
    let boat = actor(-10, "minecraft:boat", [3.5, 64.0, 3.5], ("LinksTag", Value::List(List { tag: 10, items: vec![link] })));
    let pig = actor(-20, "minecraft:pig", [3.5, 64.5, 3.5], ("Variant", Value::Int(0)));
    let mut h = Harness::with(config(), World::with_storage(floor_with([boat, pig].concat())));
    let sent = enter_stored(&mut h, 1);
    let sent = [sent, ticks(&mut h, 5)].concat();
    assert_eq!(links(&sent).len(), 1, "the pig rides the boat: {:?}", links(&sent));
    h.remote.run(|ctx| {
        let all = ctx.entities();
        let kind_of = |name: &str| all.iter().find(|entity| entity.kind == name).map(|entity| entity.id).expect(name);
        assert_eq!(ctx.riders(kind_of("minecraft:boat")), vec![Target::Entity(kind_of("minecraft:pig"))]);
    });
    h.tick(1);
}
