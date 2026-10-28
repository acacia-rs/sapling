use acacia_proto::{Packet, RawPacket};

use super::*;

fn kinds(packets: &[Bytes]) -> Vec<u32> {
    packets.iter().map(|packet| RawPacket::parse(packet.clone()).unwrap().id).collect()
}

fn axes(packet: &Bytes) -> [bool; 3] {
    let delta: MoveEntityDelta = RawPacket::parse(packet.clone()).unwrap().decode().unwrap();
    [delta.x.is_some(), delta.y.is_some(), delta.z.is_some()]
}

#[test]
fn a_move_counts_all_axes_together_and_sends_each_that_changed() {
    // BDS's orb: no axis past 0.01, together past it, all three sent.
    let mut told = Told::new([6.0, -19.0, 8.0]);
    let sent = told.update(1, [5.99276, -19.00726, 7.99742], [-0.007, -0.007, -0.002], false);
    assert_eq!(axes(&sent[0]), [true, true, true]);
    assert_eq!(axes(&told.update(1, [5.99276, -19.02, 7.99742], [0.0, -0.01, 0.0], false)[0]), [false, true, false]);
}

#[test]
fn an_all_but_stopped_body_is_sent_its_last_small_move() {
    // An item at the bottom of its bob on BDS: 0.007 sent as it turned.
    let mut told = Told::new([0.5, 60.0, 0.5]);
    assert!(told.update(1, [0.5, 59.993, 0.5], [0.0, -0.002, 0.0], false).is_empty());
    assert_eq!(kinds(&told.update(1, [0.5, 59.993, 0.5], [0.0, 0.0005, 0.0], false)), [MoveEntityDelta::ID]);
    assert!(told.update(1, [0.5, 59.9935, 0.5], [0.0, 0.0005, 0.0], false).is_empty());
}

#[test]
fn small_moves_are_kept_and_motion_is_sent_when_it_changed_a_lot_or_stopped() {
    let mut told = Told::new([0.5, 70.0, 0.5]);
    assert!(told.update(1, [0.5, 69.995, 0.5], [0.0, -0.1, 0.0], false).is_empty());
    assert_eq!(kinds(&told.update(1, [0.5, 69.9, 0.5], [0.0, -0.19, 0.0], false)), [MoveEntityDelta::ID]);
    assert_eq!(kinds(&told.update(1, [0.5, 69.6, 0.5], [0.0, -0.25, 0.0], false)), [MoveEntityDelta::ID, SetEntityMotion::ID]);
    assert_eq!(kinds(&told.update(1, [0.5, 69.0, 0.5], [0.0; 3], true)), [MoveEntityDelta::ID, SetEntityMotion::ID]);
}

#[test]
fn a_turn_is_told_past_4_degrees_and_a_far_viewer_now_and_then_as_on_bds() {
    let mut told = Told::new([0.0; 3]);
    told.face(2.0, 2.0);
    assert!(told.update(1, [0.0; 3], [0.0; 3], true).is_empty(), "2.8 degrees in bytes");
    told.face(3.0, 3.0);
    assert_eq!(kinds(&told.update(1, [0.0; 3], [0.0; 3], true)), [MoveEntityDelta::ID]);
    let mut world = bevy_ecs::world::World::new();
    let (near, far, limits) = (world.spawn_empty().id(), world.spawn_empty().id(), pace::limits("minecraft:slime"));
    let eyes = |viewer: Entity| if viewer == near { [0.0; 3] } else { [68.0, 0.0, 0.0] };
    let mut sent = Vec::new();
    for now in 1..=10 {
        told.update(1, [now as f32, 0.0, 0.0], [1.0, 0.0, 0.0], now > 5);
        sent.push([near, far].map(|viewer| told.told_to(viewer, eyes(viewer), now, limits)));
        told.settle(false);
    }
    let far_told: Vec<Option<u64>> = sent.iter().map(|[_, far]| *far).collect();
    // The first move goes to all at once, as does the landing at 6; else 8 ticks apart, 68 away.
    assert_eq!(far_told, [Some(0), None, None, None, None, Some(0), None, None, None, None]);
    assert!(sent.iter().all(|[near, _]| near.is_some()));
}

#[test]
fn a_move_told_to_no_one_is_compared_again_from_what_they_knew() {
    let mut told = Told::new([0.0; 3]);
    told.update(1, [0.006, 0.0, 0.0], [0.006, 0.0, 0.0], true);
    told.update(1, [0.012, 0.0, 0.0], [0.006, 0.0, 0.0], true);
    told.settle(true);
    assert!(!told.update(1, [0.012, 0.0, 0.0], [0.0; 3], true).is_empty(), "0.012 from 0, not 0 from 0.012");
}

#[test]
fn moving_is_flagged_a_tick_after_a_move_of_more_than_0_0316_as_on_bds() {
    let mut told = Told::new([0.0; 3]);
    assert!(told.flags(1, [0.0; 3], false, false).is_none(), "only for kinds that asked");
    told.flag_moving(false);
    // BDS's item 18 rising in water: 0.000991 squared stays off, 0.001001 shows the next tick.
    let moves = [[0.0, 0.2, 0.0], [0.0107, 0.0011, 0.0], [0.00155, 0.03148, 0.0], [0.00155, 0.0316, 0.0], [0.0, 0.0316, 0.0]];
    let sent: Vec<bool> = moves.into_iter().map(|gone| told.flags(1, gone, false, false).is_some()).collect();
    assert_eq!(sent, [false, true, true, false, true]);
    let mut fallen = Told::new([0.0; 3]);
    fallen.flag_moving(true);
    assert!(fallen.flags(1, [0.0; 3], false, false).is_some(), "a falling block is flagged as it is shown");
}

fn sent_flags(packet: &Bytes) -> i64 {
    let data: SetEntityData = RawPacket::parse(packet.clone()).unwrap().decode().unwrap();
    match data.metadata[0].value {
        Value::Flags(flags) => flags.0,
        ref other => panic!("{other:?}"),
    }
}

#[test]
fn a_mob_shows_moving_a_tick_after_it_walks_and_keeps_the_flags_set_meanwhile() {
    let announced = Flags(Flags::HAS_COLLISION.0 | Flags::WALKER.0);
    let mut told = Told::new([0.0; 3]);
    assert_eq!(told.set_flag(Flags::ONFIRE, true), None, "no mob yet");
    told.flag_mob(announced);
    assert!(told.flags(1, [0.1, 0.0, 0.0], false, false).is_none(), "the tick it walks");
    assert_eq!(sent_flags(&told.flags(1, [0.1, 0.0, 0.0], false, false).unwrap()), announced.0 | Flags::MOVING.0);
    assert_eq!(told.set_flag(Flags::ONFIRE, true), Some(Flags(announced.0 | Flags::MOVING.0 | Flags::ONFIRE.0)));
    assert!(told.flags(1, [0.0; 3], false, false).is_none(), "still moving: it moved last tick");
    assert_eq!(sent_flags(&told.flags(1, [0.0; 3], false, false).unwrap()), announced.0 | Flags::ONFIRE.0);
}

#[test]
fn a_lifted_item_loses_its_gravity_and_one_in_lava_burns_as_on_bds() {
    let mut told = Told::new([0.0; 3]);
    told.flag_moving(true);
    // BDS's item into water, then lava: 281492156579840 and 281492156579841.
    assert_eq!(sent_flags(&told.flags(1, [0.0, -0.1, 0.0], true, false).unwrap()), 281_492_156_579_840);
    assert_eq!(sent_flags(&told.flags(1, [0.0, -0.1, 0.0], true, true).unwrap()), 281_492_156_579_841);
    assert!(told.flags(1, [0.0, -0.1, 0.0], true, true).is_none());
}
