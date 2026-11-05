//! A blaze's fireballs that meet a block light fire only while `mobGriefing` is on
//! (research/captures/bds-u1blaze.txt: four fires in 15 s with it, none in 15 s without).

use crate::common;

use common::{Harness, all};
use sapling_api::{Block, Difficulty, Effect, PlayerId, RuleValue};

/// Fires about after a blaze shot 1200 ticks at a warded player behind a wall of bottom slabs, a
/// row of stone behind them for a fire to stand on.
fn fires(griefing: bool) -> usize {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        assert!(ctx.set_game_rule("doFireTick", RuleValue::Bool(false)));
        assert!(ctx.set_game_rule("mobGriefing", RuleValue::Bool(griefing)));
        for effect in [Effect::FireResistance, Effect::Resistance] {
            assert!(ctx.give_effect(PlayerId(1), effect, 2000, 4, false));
        }
        for x in -3..=3 {
            // Its fireballs' rays pass between sapling's pane posts: a slab the blaze sees over stops them.
            assert!(ctx.set_block([x, 66, 2], &Block::new("minecraft:smooth_stone_slab")));
            assert!(ctx.set_block([x, 65, 3], &Block::new("minecraft:stone")));
        }
        assert!(ctx.spawn_entity("minecraft:blaze", [0.5, 65.0, 6.5]).is_some());
    });
    let shots: usize = (0..1200).map(|_| all::<acacia_proto::packets::AddEntity>(h.tick_all().to(1)).iter().filter(|added| added.entity_type == "minecraft:small_fireball").count()).sum();
    assert!(shots > 0, "the blaze shot");
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| {
        let cells = (-6..=6).flat_map(|x| (-6..=12).flat_map(move |z| (64..=67).map(move |y| [x, y, z])));
        sent.send(cells.filter(|at| ctx.block(*at).is_some_and(|block| block.name == "minecraft:fire")).count()).expect("the test waits")
    });
    h.tick_all();
    got.recv().expect("ran")
}

#[test]
fn a_blazes_fireballs_light_fire_only_with_mob_griefing() {
    assert!(fires(true) > 0, "a fireball that met the panes lights the stone behind");
    assert_eq!(fires(false), 0);
}
