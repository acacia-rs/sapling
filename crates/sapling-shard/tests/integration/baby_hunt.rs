//! A baby zombie hunts at its own pace, as BDS's did (research/captures/bds-q3baby.txt).

use crate::common;

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use common::{Harness, all};
use sapling_api::{Difficulty, GameMode, PlayerId};

#[test]
fn a_baby_zombie_7_off_hits_within_45_ticks_of_its_summon() {
    // BDS's three, summoned 7 north of the actor, first hit 43, 30 and 32 ticks after they came; at a
    // grown zombie's 0.23 sapling's took 60.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.receive(2, &common::input_at([8.5, 65.0, 8.5]));
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        ctx.set_game_mode(PlayerId(2), GameMode::Creative);
        assert!(ctx.summon_as(None, "minecraft:zombie", [0.5, 65.0, -6.5], 0.0, 0.0, Some("minecraft:as_baby")).is_some());
    });
    let hit = (0..80).position(|_| all::<EntityEvent>(h.tick_all().to(2)).iter().any(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation));
    assert!(hit.is_some_and(|tick| (25..=45).contains(&tick)), "first hit at {hit:?}");
}
