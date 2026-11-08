//! Riding: seats taken by a click and by the API, left on `LeaveVehicle`, told to viewers with
//! `SetEntityLink` (research/captures/bds-mounts.txt), saddles, and links kept in a world folder.

use std::sync::{Arc, Mutex};

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::{List, Value};
use acacia_proto::Packet;
use acacia_proto::packets::{AddEntity, AddPlayer, EntityEvent, EntityEventEventId as Event, Interact, InteractActionId, InventoryTransaction, SetEntityData, SetEntityLink};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, TransactionTransactionData, TransactionTransactionDataItemUseOnEntityActionType as OnEntity};
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
    // The second seating tells the first rider's link again, then its own (bds-ride.txt).
    assert_eq!(links(&sent).iter().filter(|(ridden, ..)| *ridden == boat.0 as i64).count(), 3, "{:?}", links(&sent));
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

fn riders(h: &mut Harness, of: EntityId) -> usize {
    let count = Arc::new(Mutex::new(0));
    let slot = count.clone();
    h.remote.run(move |ctx| *slot.lock().unwrap() = ctx.riders(of).len());
    h.tick(1);
    let riders = *count.lock().unwrap();
    riders
}

#[test]
fn a_wild_horse_throws_its_rider_off_until_its_temper_lets_a_ride_tame_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let horse = spawned(&mut h, &[("minecraft:horse", [2.5, 65.0, 0.5])])[0];
    h.receive(1, &interact(horse.0));
    // It lets its rider stay 60 ticks before it rolls (research/captures/bds-n3ride.txt).
    ticks(&mut h, 57);
    assert_eq!(riders(&mut h, horse), 1, "still on before its 60th ridden tick");
    ticks(&mut h, 3);
    // Its first temper is rolled; each failed ride adds 5, of 100: by the 20th the roll cannot fail.
    let tamed = riders(&mut h, horse) == 1 || (0..20).any(|_| {
        h.receive(1, &interact(horse.0));
        ticks(&mut h, 62);
        riders(&mut h, horse) == 1
    });
    assert!(tamed, "a ride tamed it and its rider stays on");
}

/// The keys of each `SetEntityData` about `id`.
fn data_keys(sent: &[RawPacket], id: u64) -> Vec<Vec<Key>> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).map(|data| data.metadata.iter().map(|item| item.key).collect()).collect()
}

#[test]
fn a_horse_tamed_by_a_ride_shows_its_owner_with_the_flag_and_its_inventory_the_tick_after() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let horse = spawned(&mut h, &[("minecraft:horse", [2.5, 65.0, 0.5])])[0];
    for _ in 0..21 {
        h.receive(1, &interact(horse.0));
        for _ in 0..62 {
            let sent = h.tick(1);
            if !all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == horse.0 && event.event_id == Event::TameSuccess) {
                continue;
            }
            // research/captures/bds-n3ride.txt: the word and OwnerEid, then the inventory alone.
            assert_eq!(data_keys(&sent, horse.0), [vec![Key::Flags, Key::OwnerEid]]);
            assert_eq!(data_keys(&h.tick(1), horse.0), [vec![Key::ContainerType, Key::ContainerBaseSize]]);
            return;
        }
    }
    panic!("no ride tamed the horse");
}

#[test]
fn a_boat_kept_with_its_rider_takes_it_back_when_loaded() {
    let link = compound([("entityID", Value::Long(-20)), ("linkID", Value::Int(0))]);
    let boat = actor(-10, "minecraft:boat", [3.5, 64.0, 3.5], ("LinksTag", Value::List(List { tag: 10, items: vec![link] })));
    let pig = actor(-20, "minecraft:pig", [3.5, 64.5, 3.5], ("Variant", Value::Int(0)));
    let mut h = Harness::with(config(), World::with_storage(floor_with([boat, pig].concat())));
    let sent = enter_stored(&mut h, 1);
    let sent = [sent, ticks(&mut h, 5)].concat();
    // Told after the pig's announcement and again after the boat's, as BDS's riders (bds-h3herds.txt).
    let told = links(&sent);
    assert!(!told.is_empty() && told.iter().all(|link| *link == told[0] && link.2 == 1), "the pig rides the boat: {told:?}");
    h.remote.run(|ctx| {
        let all = ctx.entities();
        let kind_of = |name: &str| all.iter().find(|entity| entity.kind == name).map(|entity| entity.id).expect(name);
        assert_eq!(ctx.riders(kind_of("minecraft:boat")), vec![Target::Entity(kind_of("minecraft:pig"))]);
    });
    h.tick(1);
}

/// research/captures/bds-m3rideview.txt: a player coming to see a boat's player rider got its
/// `AddPlayer` with the seat, its link, then the boat.
#[test]
fn a_late_viewer_gets_a_player_rider_seated_then_its_link_then_the_boat() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let boat = spawned(&mut h, &[("minecraft:boat", [2.5, 65.0, 0.5])])[0];
    h.receive(1, &interact(boat.0));
    ticks(&mut h, 2);
    let mut seen = h.enter(2, "Bob").to(2).to_vec();
    (0..3).for_each(|_| seen.extend_from_slice(h.tick_all().to(2)));
    let at = |id: u32| seen.iter().position(|p| p.id == id);
    let alex = at(AddPlayer::ID).expect("Alex shown");
    let player: AddPlayer = seen[alex].decode().unwrap();
    assert!(player.metadata.iter().any(|item| item.key == Key::RiderSeatPosition), "the seat rides in the AddPlayer");
    assert_eq!(links(&seen[alex + 1..alex + 2]), [(boat.0 as i64, 1, 1)], "the link right after it");
    assert!(at(AddEntity::ID).is_some_and(|added| added > alex + 1), "the boat after");
    assert_eq!(links(&seen).len(), 1, "told once");
}

/// The `FlagsExtended` of player `id` in each `SetEntityData` sent and `AddPlayer`, as bits 8 and 9.
fn guard_bits(sent: &[RawPacket], id: u64) -> Vec<i64> {
    use acacia_proto::types::MetadataDictionaryItemValue as Meta;
    let of = |metadata: &[acacia_proto::types::MetadataDictionaryItem]| metadata.iter().find_map(|item| if let Meta::FlagsExtended(flags) = &item.value { Some(flags.0 >> 8 & 3) } else { None });
    let data = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).filter_map(|data| of(&data.metadata));
    data.chain(all::<AddPlayer>(sent).into_iter().filter(|added| added.runtime_id == id).filter_map(|added| of(&added.metadata))).collect()
}

/// research/captures/bds-m3rideview.txt: a ride start set bits 8 and 9 (the shield's raised and
/// moving), its end cleared 8, and 9 stayed, in a later viewer's `AddPlayer` too.
#[test]
fn a_rider_is_shown_with_its_shield_bits_and_the_moving_one_stays_after() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let boat = spawned(&mut h, &[("minecraft:boat", [2.5, 65.0, 0.5])])[0];
    h.enter(2, "Bob");
    h.tick_all();
    h.receive(1, &interact(boat.0));
    let seen: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    assert_eq!(guard_bits(&seen, 1), [3]);
    h.receive(1, &Interact { action_id: InteractActionId::LeaveVehicle, target_entity_id: boat.0, has_position: false, position: None });
    let seen: Vec<RawPacket> = (0..4).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    let bits = guard_bits(&seen, 1);
    assert!(bits.first() == Some(&2) && bits.iter().all(|bits| *bits == 2), "{bits:?}");
    let late = h.enter(3, "Cleo").to(3).to_vec();
    let late: Vec<RawPacket> = late.into_iter().chain((0..3).flat_map(|_| h.tick_all().to(3).to_vec())).collect();
    assert_eq!(guard_bits(&late, 1), [2]);
}
