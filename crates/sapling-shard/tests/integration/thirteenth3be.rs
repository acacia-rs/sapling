//! Group BE's actor order (research/captures/bds-be3slots.txt; entities/slots.rs): what mobs' changes are
//! told at the tick's end goes out highest storage slot first, and a new actor takes the slot freed last.

use crate::common;

use acacia_proto::packets::{MobEffect, SetEntityData};
use common::shooting::{ready, spawn, watch};
use common::{all, asking::ask};
use sapling_api::{Effect, EntityId, GameMode};

/// The mobs each SetEntityData Alex is told of in the tick `event` is fired on `ids`, in that order, is about.
fn told_order(h: &mut common::Harness, ids: &[u64], event: &'static str) -> Vec<u64> {
    let ids = ids.to_vec();
    h.remote.run(move |ctx| ids.iter().for_each(|id| assert!(ctx.fire_entity_event(EntityId(*id), event))));
    all::<SetEntityData>(h.tick_all().to(1)).into_iter().map(|data| data.runtime_entity_id).collect()
}

#[test]
fn tick_end_changes_go_highest_slot_first_and_a_new_mob_takes_the_slot_freed_last() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let [a, b, c, d] = [[3.5, 65.0, 0.5], [-4.5, 65.0, 2.5], [1.5, 65.0, -5.5], [6.5, 65.0, 6.5]].map(|at| spawn(&mut h, "minecraft:sheep", at));
    watch(&mut h, 2);
    assert_eq!(told_order(&mut h, &[a, b, c, d], "wololo"), [d, c, b, a], "newest first while none has gone");
    for gone in [b, c] {
        assert!(ask(&mut h, move |ctx| ctx.remove_entity(EntityId(gone))));
    }
    let [e, f, g] = [[-2.5, 65.0, -7.5], [8.5, 65.0, -1.5], [-7.5, 65.0, 7.5]].map(|at| spawn(&mut h, "minecraft:sheep", at));
    watch(&mut h, 2);
    assert_eq!(told_order(&mut h, &[a, d, e, f, g], "minecraft:on_sheared"), [g, d, e, f, a], "e in c's slot, f in b's");
    h.remote.run(move |ctx| [a, f, e].iter().for_each(|id| assert!(ctx.give_entity_effect(EntityId(*id), Effect::Speed, 100, 0))));
    let sent = h.tick_all().to(1).to_vec();
    let given: Vec<u64> = all::<MobEffect>(&sent).into_iter().map(|effect| effect.runtime_entity_id).collect();
    let shown: Vec<u64> = all::<SetEntityData>(&sent).into_iter().map(|data| data.runtime_entity_id).collect();
    assert_eq!((given, shown), (vec![a, f, e], vec![e, f, a]), "MobEffect as given, VisibleMobEffects in the sync's order");
}
