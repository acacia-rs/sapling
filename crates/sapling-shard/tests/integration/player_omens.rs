//! What the four newest effects do to a player that dies or is hurt with them
//! (research/captures/bds-player-omens-findings.md).

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::{AddEntity, LevelEvent, LevelEventEvent, SetEntityMotion};
use common::{Harness, all};
use sapling_api::{DamageCause, Effect, GameMode, PlayerId};

/// A creative watcher and a survival player 4 off with `effect` on it; what the watcher is sent
/// as that player is killed.
fn killed_with(effect: Effect) -> (Harness, Vec<acacia_proto::RawPacket>) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        ctx.set_game_mode(PlayerId(2), GameMode::Survival);
        ctx.teleport(PlayerId(2), [4.5, 65.0, 0.5]);
        assert!(ctx.give_effect(PlayerId(2), effect, 1200, 0, true));
    });
    drop(h.tick_all());
    h.remote.run(|ctx| assert!(ctx.kill_player(PlayerId(2))));
    let sent: Vec<acacia_proto::RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    (h, sent)
}

#[test]
fn a_wind_charged_player_bursts_at_its_eyes_as_it_dies() {
    let (_, sent) = killed_with(Effect::WindCharged);
    let burst = all::<LevelEvent>(&sent).into_iter().find(|event| event.event == LevelEventEvent::ParticleWindExplosion).map(|event| (event.position.y, event.data));
    let pushed = all::<SetEntityMotion>(&sent).iter().find(|motion| motion.runtime_entity_id == 1).map(|motion| motion.velocity.x);
    assert!(burst.is_some_and(|(y, data)| (y - 66.72).abs() < 1e-3 && data == 3), "burst {burst:?}");
    assert!(pushed.is_some_and(|x| x < -0.2), "pushed {pushed:?}");
}

#[test]
fn a_weaving_player_leaves_cobwebs_and_an_oozing_one_slimes() {
    let (mut h, _) = killed_with(Effect::Weaving);
    let webs = Arc::new(Mutex::new(0));
    let counted = webs.clone();
    h.remote.run(move |ctx| {
        let cells = (3..=5).flat_map(|x| (65..=67).flat_map(move |y| (-1..=1).map(move |z| [x, y, z])));
        *counted.lock().unwrap() = cells.filter(|cell| ctx.block(*cell).is_some_and(|block| block.name == "minecraft:web")).count();
    });
    // A remote call runs in the shard's next tick.
    drop(h.tick_all());
    let webs = *webs.lock().unwrap();
    assert!((2..=3).contains(&webs), "{webs} cobwebs");
    let (_, sent) = killed_with(Effect::Oozing);
    let slimes: Vec<f32> = all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:slime").map(|added| added.position.y).collect();
    assert_eq!(slimes.len(), 2);
}

#[test]
fn an_infested_player_hurt_lets_out_silverfish_now_and_then() {
    // A time in ten a hurt: forty hurts, mended between, let none out but one time in sixty.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Survival);
        assert!(ctx.give_effect(PlayerId(1), Effect::Infested, 6000, 0, true));
    });
    drop(h.tick_all());
    let mut silverfish = 0;
    for _ in 0..40 {
        h.remote.run(|ctx| assert!(ctx.hurt_player(PlayerId(1), 1.0, DamageCause::Generic)));
        for _ in 0..12 {
            silverfish += all::<AddEntity>(h.tick_all().to(1)).iter().filter(|added| added.entity_type == "minecraft:silverfish").count();
        }
        h.remote.run(|ctx| assert!(ctx.give_effect(PlayerId(1), Effect::InstantHealth, 1, 1, true)));
        drop(h.tick_all());
    }
    assert!(silverfish >= 1, "no silverfish in forty hurts");
}
