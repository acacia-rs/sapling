//! Guardians' lasers and an elder's curse, against research/captures/bds-guardians.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, MobEffect, SetEntityData, UpdateAttributes};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all};
use sapling_api::{Block, Difficulty};

/// Alex in survival on `difficulty` by a one-deep pool, glass-rimmed, with a `kind` in it; its id.
fn by_a_pool(kind: &'static str, difficulty: Difficulty) -> (Harness, u64) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(difficulty);
        for x in 2_i32..=8 {
            for z in -3_i32..=3 {
                let rim = x == 2 || x == 8 || z.abs() == 3;
                assert!(ctx.set_block([x, 65, z], &Block::new(if rim { "glass" } else { "water" })));
            }
        }
        assert!(ctx.spawn_entity(kind, [5.5, 65.0, 0.5]).is_some());
    });
    let added = (0..2).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).find(|added| added.entity_type == kind).expect("shown");
    (h, added.runtime_id)
}

/// What Alex is sent over `ticks` ticks, by tick.
fn watch(h: &mut Harness, ticks: usize) -> Vec<Vec<RawPacket>> {
    (0..ticks).map(|_| h.tick_all().to(1).to_vec()).collect()
}

/// The first tick in `ticks` at which `found` holds of what was sent.
fn first(ticks: &[Vec<RawPacket>], found: impl Fn(&[RawPacket]) -> bool) -> Option<usize> {
    ticks.iter().position(|sent| found(sent))
}

fn targets(sent: &[RawPacket], id: u64) -> Vec<i64> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata).filter(|item| item.key == Key::TargetEid).filter_map(|item| if let Value::Default(Plain::Long(target)) = item.value { Some(target) } else { None }).collect()
}

fn health(sent: &[RawPacket]) -> Vec<f32> {
    all::<UpdateAttributes>(sent).into_iter().filter(|update| update.runtime_entity_id == 1).flat_map(|update| update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect()
}

#[test]
fn a_guardian_takes_a_player_shows_its_laser_9_ticks_on_and_lands_it_80_after_for_5() {
    let (mut h, guardian) = by_a_pool("minecraft:guardian", Difficulty::Normal);
    let ticks = watch(&mut h, 120);
    let took = first(&ticks, |sent| targets(sent, guardian) == [1]).expect("it takes Alex");
    let shown = first(&ticks, |sent| all::<EntityEvent>(sent).iter().any(|event| event.runtime_entity_id == guardian && event.event_id == EntityEventEventId::GuardianAttack)).expect("its laser");
    let landed = first(&ticks, |sent| !health(sent).is_empty()).expect("its hit");
    assert_eq!((shown - took, landed - shown), (9, 80));
    // research/captures/bds-guardianhits.txt: 20 to 15; the magic is guarded by the hit.
    assert_eq!(health(&ticks[landed]).last(), Some(&15.0), "{:?}", health(&ticks[landed]));
    assert_eq!(targets(&ticks[landed], guardian), [0], "it lets go as it lands");
}

#[test]
fn an_elder_guardian_curses_a_survival_player_near_it_with_six_minutes_of_mining_fatigue_iii() {
    let (mut h, _) = by_a_pool("minecraft:elder_guardian", Difficulty::Easy);
    let ticks = watch(&mut h, 101);
    let at = first(&ticks, |sent| all::<MobEffect>(sent).iter().any(|effect| effect.runtime_entity_id == 1 && effect.effect_id == 4)).expect("cursed within 100 ticks");
    let effect = all::<MobEffect>(&ticks[at]).into_iter().find(|effect| effect.effect_id == 4).expect("found above");
    assert_eq!((effect.amplifier, effect.duration, effect.particles), (2, 6000, true));
    assert!(all::<EntityEvent>(&ticks[at]).iter().any(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::ElderGuardianCurse));
    let again = watch(&mut h, 101);
    assert!(again.iter().all(|sent| all::<MobEffect>(sent).iter().all(|effect| effect.effect_id != 4)), "not again while it has a minute or more of it");
}
