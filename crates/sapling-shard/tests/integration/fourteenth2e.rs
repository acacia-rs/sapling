//! Group E of the fourteenth wave, water mobs (research/captures/bds-f14eguard.txt, bds-f14eguard2.txt,
//! bds-f14esquid.txt, bds-f14edolphin.txt, bds-f14edolphin2.txt; creatures/{guardians,squids,dolphins}.rs).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, MoveEntityDelta, SetEntityData, UpdateAttributes};
use acacia_proto::types::{MetadataFlags1, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all, asking::ask, attack};
use sapling_api::{Block, Difficulty};

/// Alex in survival on normal beside a glass-walled pool at x -3..4, z 2..9, water from 60 to 66,
/// with a `kind` in it at `at`; its id.
fn pool_with(kind: &'static str, at: [f32; 3]) -> (Harness, u64) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    pool_with_in(h, kind, at)
}

fn pool_with_in(mut h: Harness, kind: &'static str, at: [f32; 3]) -> (Harness, u64) {
    h.remote.run(move |ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        for x in -3_i32..=4 {
            for z in 1_i32..=10 {
                for y in 59_i32..=66 {
                    let rim = x == -3 || x == 4 || z == 1 || z == 10 || y == 59;
                    assert!(ctx.set_block([x, y, z], &Block::new(if rim { "glass" } else { "water" })));
                }
            }
        }
        assert!(ctx.spawn_entity(kind, at).is_some());
    });
    let added = (0..2).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).find(|added| added.entity_type == kind).expect("shown");
    (h, added.runtime_id)
}

fn targets(sent: &[RawPacket], id: u64) -> Vec<i64> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata).filter(|item| item.key == Key::TargetEid).filter_map(|item| if let Value::Default(Plain::Long(target)) = item.value { Some(target) } else { None }).collect()
}

fn health(sent: &[RawPacket], id: u64) -> Vec<f32> {
    all::<UpdateAttributes>(sent).into_iter().filter(|update| update.runtime_entity_id == id).flat_map(|update| update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect()
}

/// As [`pool_with`], Alex standing in the pool's near end, as BDS's actor was (the rim would hide him).
fn in_pool_with(kind: &'static str, at: [f32; 3]) -> (Harness, u64) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.receive(1, &common::input_at([0.5, 61.0, 2.5]));
    drop(h.tick_all());
    pool_with_in(h, kind, at)
}

#[test]
fn a_guardian_aiming_still_spikes_its_hitter_for_2() {
    // Its feet 3.16 from Alex's eyes, outside the 3 it lets prey go within.
    let (mut h, guardian) = in_pool_with("minecraft:guardian", [0.5, 62.0, 5.6]);
    let took = (0..40).position(|_| targets(h.tick_all().to(1), guardian) == [1]);
    assert!(took.is_some(), "it takes Alex");
    // Its way stops as it aims: a few ticks for its drift to die and `MOVING` to clear.
    drop(ask(&mut h, |_| ()));
    for _ in 0..10 {
        drop(h.tick_all());
    }
    h.receive(1, &attack(guardian));
    let sent: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(health(&sent, guardian).contains(&29.0), "the hit lands: {:?}", health(&sent, guardian));
    let alex_flashes: Vec<i32> = all::<EntityEvent>(&sent).iter().filter(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation).map(|event| event.data).collect();
    assert_eq!(health(&sent, 1), [18.0], "spiked for 2, flashes {alex_flashes:?}");
}

#[test]
fn a_guardian_lets_prey_within_3_go_the_tick_after_and_takes_none_for_a_second() {
    // bds-f14eguard.txt: taken at 22522, let go at 22576; taken again 60-64 ticks on.
    let (mut h, guardian) = in_pool_with("minecraft:guardian", [0.5, 62.0, 4.6]);
    let told: Vec<Vec<i64>> = (0..60).map(|_| targets(h.tick_all().to(1), guardian)).collect();
    let took = told.iter().position(|told| told == &[1]).expect("it takes Alex");
    assert_eq!(told[took + 1], [0], "let go the tick after");
    assert!(told[took + 2..(took + 20).min(told.len())].iter().all(Vec::is_empty), "none taken for 20 ticks: {told:?}");
}

#[test]
fn a_hit_squid_inks_and_flees_its_hitter_at_3_a_second() {
    let (mut h, squid) = pool_with("minecraft:squid", [0.5, 65.5, 2.7]);
    drop(ask(&mut h, |_| ()));
    h.receive(1, &attack(squid));
    let sent = h.tick_all().to(1).to_vec();
    let events: Vec<EntityEventEventId> = all::<EntityEvent>(&sent).iter().filter(|event| event.runtime_entity_id == squid).map(|event| event.event_id).collect();
    assert_eq!(events.get(..2), Some(&[EntityEventEventId::HurtAnimation, EntityEventEventId::SquidInkCloud][..]), "{events:?}");
    let ticks: Vec<Vec<RawPacket>> = (0..40).map(|_| h.tick_all().to(1).to_vec()).collect();
    let z: Vec<f32> = ticks.iter().filter_map(|sent| all::<MoveEntityDelta>(sent).into_iter().filter(|moved| moved.runtime_entity_id == squid).find_map(|moved| moved.z)).collect();
    // bds-f14esquid.txt: 0.15 a tick of way, which its speed settles at 0.585 of; mostly along z here.
    let late = z[z.len() - 10..].windows(2).map(|pair| pair[1] - pair[0]).sum::<f32>() / 9.0;
    assert!(late > 0.06, "it swims off at {late} a tick: {z:?}");
}

fn angry(sent: &[RawPacket], id: u64) -> Option<bool> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata).find_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0 & MetadataFlags1::ANGRY.0 != 0) } else { None })
}

#[test]
fn a_hit_dolphin_angers_the_dolphins_near_which_take_it_a_tick_later() {
    // bds-f14edolphin2.txt: the hit one took the hitter in the hit's tick, those near in the next;
    // each showed `ANGRY` the tick after it took it.
    let (mut h, hit) = pool_with("minecraft:dolphin", [0.5, 65.0, 2.7]);
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:dolphin", [2.5, 61.0, 8.5]).is_some()));
    let near = all::<AddEntity>(h.tick_all().to(1)).iter().find(|added| added.entity_type == "minecraft:dolphin").expect("shown").runtime_id;
    drop(ask(&mut h, |_| ()));
    h.receive(1, &attack(hit));
    let sent: Vec<Vec<RawPacket>> = (0..4).map(|_| h.tick_all().to(1).to_vec()).collect();
    for (id, took) in [(hit, 0), (near, 1)] {
        let taken: Vec<Vec<i64>> = sent.iter().map(|sent| targets(sent, id)).collect();
        assert_eq!(taken[took], [1], "{id} takes Alex in tick {took}: {taken:?}");
        let shown: Vec<Option<bool>> = sent.iter().map(|sent| angry(sent, id)).collect();
        assert_eq!(shown[took + 1], Some(true), "{id} shows its anger the tick after: {shown:?}");
    }
}

#[test]
fn a_dolphin_ashore_hops_and_dries_out_after_2400_ticks() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_block([10, 64, 0], &Block::new("stone")));
        assert!(ctx.spawn_entity("minecraft:dolphin", [10.5, 65.0, 0.5]).is_some());
    });
    let sent: Vec<Vec<RawPacket>> = (0..2420).map(|_| h.tick_all().to(1).to_vec()).collect();
    let id = sent.iter().find_map(|sent| all::<AddEntity>(sent).first().map(|added| added.runtime_id)).expect("shown");
    let ys: Vec<f32> = sent.iter().filter_map(|sent| all::<MoveEntityDelta>(sent).into_iter().filter(|moved| moved.runtime_entity_id == id).find_map(|moved| moved.y)).collect();
    // bds-f14edolphin.txt: each hop's first tick rose 0.4216.
    assert!(ys.windows(2).any(|pair| (pair[1] - pair[0] - 0.4216).abs() < 2e-3), "{:?}", &ys[..ys.len().min(30)]);
    let dried = sent.iter().position(|sent| all::<EntityEvent>(sent).iter().any(|event| event.runtime_entity_id == id && event.event_id == EntityEventEventId::HurtAnimation && event.data == -1));
    assert!(dried.is_some_and(|tick| (2398..=2410).contains(&tick)), "first dry hit at {dried:?}");
}
