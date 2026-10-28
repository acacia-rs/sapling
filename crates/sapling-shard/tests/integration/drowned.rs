//! Drowned that throw tridents, against research/captures/bds-drowned.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, MobEquipment};
use common::{Harness, all};
use sapling_api::Difficulty;

#[test]
fn a_drowned_come_as_ranged_holds_a_trident_and_throws_it_a_tick_after_its_swing() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        assert!(ctx.summon_as(None, "minecraft:drowned", [6.5, 65.0, -6.5], 0.0, Some("minecraft:as_ranged_adult")).is_some());
    });
    let ticks: Vec<Vec<RawPacket>> = (0..120).map(|_| h.tick_all().to(1).to_vec()).collect();
    let drowned = ticks.iter().find_map(|sent| all::<AddEntity>(sent).into_iter().find(|added| added.entity_type == "minecraft:drowned")).expect("shown").runtime_id;
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
