//! `effect clear` against research/captures/bds-q3witch.txt: the removals go out by effect id, not
//! in the order the effects came.

use crate::common;

use acacia_proto::packets::{MobEffect, MobEffectEventId};
use common::{Harness, all};
use sapling_api::{Effect, PlayerId};

const ALEX: PlayerId = PlayerId(1);

#[test]
fn a_clear_tells_the_removals_in_effect_id_order() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for effect in [Effect::HealthBoost, Effect::Resistance, Effect::Slowness, Effect::Poison] {
            assert!(ctx.give_effect(ALEX, effect, 600, 0, true));
        }
    });
    drop(h.tick_all());
    h.remote.run(|ctx| assert!(ctx.clear_effects(ALEX)));
    let removed: Vec<i32> = all::<MobEffect>(h.tick_all().to(1)).into_iter().filter(|effect| effect.event_id == MobEffectEventId::Remove).map(|effect| effect.effect_id).collect();
    assert_eq!(removed, [2, 11, 19, 21]);
}
