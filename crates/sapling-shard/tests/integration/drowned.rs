//! Drowned that throw tridents, against research/captures/bds-drowned.txt and bds-s3drowned.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, MobEquipment};
use common::{Harness, all};
use sapling_api::{Difficulty, EntityId};

const SPOT: [f32; 3] = [6.5, 65.0, -6.5];

/// A drowned come as ranged, held at `SPOT` 9 from Alex for `ticks` ticks (else it walks into biting
/// range); the harness, what Alex was sent each tick and the drowned's runtime id.
fn held_drowned(ticks: usize) -> (Harness, Vec<Vec<RawPacket>>, u64) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        assert!(ctx.summon_as(None, "minecraft:drowned", SPOT, 0.0, 0.0, Some("minecraft:as_ranged_adult")).is_some());
    });
    let mut sent = vec![h.tick_all().to(1).to_vec()];
    let drowned = all::<AddEntity>(&sent[0]).into_iter().find(|added| added.entity_type == "minecraft:drowned").expect("shown").runtime_id;
    for _ in 1..ticks {
        h.remote.run(move |ctx| assert!(ctx.move_entity(EntityId(drowned), SPOT)));
        sent.push(h.tick_all().to(1).to_vec());
    }
    (h, sent, drowned)
}

#[test]
fn a_drowned_come_as_ranged_holds_a_trident_and_throws_it_a_tick_after_its_swing() {
    let (mut h, ticks, drowned) = held_drowned(160);
    let held = ticks.iter().flat_map(|sent| all::<MobEquipment>(sent)).find(|held| held.runtime_entity_id == drowned).expect("shown armed");
    let held_id = held.item.network_id;
    assert_ne!(held_id, 0);
    let swung = ticks.iter().position(|sent| all::<EntityEvent>(sent).iter().any(|event| event.runtime_entity_id == drowned && event.event_id == EntityEventEventId::ArmSwing)).expect("it swings");
    let thrown = ticks.iter().position(|sent| all::<AddEntity>(sent).iter().any(|added| added.entity_type == "minecraft:thrown_trident")).expect("a trident flies");
    assert_eq!(thrown, swung + 1);

    let mut shown: Vec<RawPacket> = h.enter(2, "Steve").to(2).to_vec();
    shown.extend((0..3).flat_map(|_| h.tick_all().to(2).to_vec()));
    assert!(all::<MobEquipment>(&shown).iter().any(|held| held.runtime_entity_id == drowned && held.item.network_id == held_id), "one who comes later sees it armed");
}

/// bds-drownedloot.txt: a dead drowned that keeps its trident (no player killed it) is shown
/// holding it again, its offhand emptied.
#[test]
fn a_drowned_killed_by_no_player_is_shown_holding_the_trident_it_keeps() {
    let (mut h, ticks, drowned) = held_drowned(2);
    let held = ticks.iter().flat_map(|sent| all::<MobEquipment>(sent)).find(|held| held.runtime_entity_id == drowned).expect("shown armed").item.network_id;
    h.remote.run(move |ctx| assert!(ctx.kill_entity(EntityId(drowned))));
    let shown: Vec<MobEquipment> = all::<MobEquipment>(h.tick_all().to(1)).into_iter().filter(|shown| shown.runtime_entity_id == drowned).collect();
    let hands: Vec<(u8, i16)> = shown.iter().map(|shown| (shown.slot, shown.item.network_id)).collect();
    assert_eq!(hands, [(0, held), (1, 0)]);
}

/// bds-s3drowned.txt: throws 48 to 84 ticks apart whatever the distance, the first 72 to 93 after
/// it took its prey; not the pack's 20 to 60.
#[test]
fn a_drowned_throws_48_to_84_ticks_apart() {
    let (_, ticks, _) = held_drowned(600);
    let thrown: Vec<usize> = ticks.iter().enumerate().filter(|(_, sent)| all::<AddEntity>(sent).iter().any(|added| added.entity_type == "minecraft:thrown_trident")).map(|(tick, _)| tick).collect();
    assert!(thrown.len() >= 4, "{thrown:?}");
    assert!(thrown[0] >= 68, "the first throw waits a second of sight and a full wait: {thrown:?}");
    assert!(thrown.windows(2).all(|pair| pair[1] - pair[0] >= 48 && pair[1] - pair[0] <= 85), "{thrown:?}");
}
