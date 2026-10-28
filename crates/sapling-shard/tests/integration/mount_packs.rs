//! A tamed mount's inventory (riding/packs.rs, docs/research/mob-life.md "Mount inventories"):
//! a chest put on a donkey, and its screen opened by its rider's inventory key and by a sneaking click.

use std::sync::{Arc, Mutex};

use crate::common;
use crate::riding::{interact, spawned};

use acacia_proto::RawPacket;
use acacia_proto::packets::{ContainerOpen, Interact, InteractActionId, LevelSoundEvent, UpdateEquipment};
use acacia_proto::types::{InputData, WindowType};
use common::{Harness, all, input_at};
use sapling_api::{EntityId, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

fn ticks(h: &mut Harness, count: usize) -> Vec<RawPacket> {
    (0..count).flat_map(|_| h.tick(1)).collect()
}

fn riders(h: &mut Harness, of: EntityId) -> usize {
    let count = Arc::new(Mutex::new(0));
    let slot = count.clone();
    h.remote.run(move |ctx| *slot.lock().unwrap() = ctx.riders(of).len());
    h.tick(1);
    let riders = *count.lock().unwrap();
    riders
}

fn ask(action_id: InteractActionId, target: u64) -> Interact {
    Interact { action_id, target_entity_id: target, has_position: false, position: None }
}

/// The horse screens Alex was shown in `sent`: the mob each names and how many slots it has.
fn screens(sent: &[RawPacket]) -> Vec<(i64, u8)> {
    let opened: Vec<ContainerOpen> = all::<ContainerOpen>(sent).into_iter().filter(|open| open.window_type == WindowType::Horse).collect();
    let equipped = all::<UpdateEquipment>(sent);
    opened.iter().filter_map(|open| equipped.iter().find(|told| told.window_id == open.window_id && told.window_type == WindowType::Horse).map(|told| (open.runtime_entity_id, told.size))).collect()
}

#[test]
fn a_tamed_donkey_opens_to_its_rider_takes_a_chest_and_opens_to_a_sneaking_click() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let donkey = spawned(&mut h, &[("minecraft:donkey", [2.5, 65.0, 0.5])])[0];
    let tamed = (0..25).any(|_| {
        h.receive(1, &interact(donkey.0));
        ticks(&mut h, 2);
        riders(&mut h, donkey) == 1
    });
    assert!(tamed, "a ride tamed it");
    h.receive(1, &ask(InteractActionId::OpenInventory, 1));
    let sent = ticks(&mut h, 2);
    assert_eq!(screens(&sent), [(donkey.0 as i64, 2)], "its saddle and armour slots");

    h.receive(1, &ask(InteractActionId::LeaveVehicle, donkey.0));
    ticks(&mut h, 2);
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:chest", 1)))));
    h.tick(1);
    h.receive(1, &interact(donkey.0));
    let sent = ticks(&mut h, 2);
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "add.chest"));
    h.remote.run(move |ctx| {
        assert!(ctx.item(ALEX, Slot::Main(0)).is_none(), "the chest went on");
        assert!(ctx.riders(donkey).is_empty(), "a chest put on is all the click does");
    });
    h.tick(1);

    let feet = Arc::new(Mutex::new([0.0; 3]));
    let slot = feet.clone();
    h.remote.run(move |ctx| *slot.lock().unwrap() = ctx.position(ALEX).expect("Alex").feet);
    h.tick(1);
    let feet = *feet.lock().unwrap();
    let mut input = input_at(feet);
    // Getting off moved Alex: the client owns up to that teleport first.
    input.input_data = vec![InputData::HandledTeleport, InputData::StartSneaking];
    h.receive(1, &input);
    h.tick(1);
    h.receive(1, &interact(donkey.0));
    let sent = ticks(&mut h, 2);
    assert_eq!(screens(&sent), [(donkey.0 as i64, 17)], "tack and 15 chest slots");
    assert_eq!(riders(&mut h, donkey), 0, "a sneaking click opens it and does not ride");
}
