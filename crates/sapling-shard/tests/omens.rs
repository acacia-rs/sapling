//! What the four newest effects do to a mob that dies or is hurt with them
//! (research/captures/bds-effects3-findings.md).

mod common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::{AddEntity, LevelEvent, LevelEventEvent, SetEntityMotion};
use common::{Harness, all};
use sapling_api::{Effect, EntityId, GameMode, PlayerId};

/// A creative player and a zombie 4 off with `effect` on it; what is sent as the zombie is killed.
fn killed_with(effect: Effect) -> (Harness, Vec<acacia_proto::RawPacket>) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let zombie = Arc::new(Mutex::new(None));
    let found = zombie.clone();
    h.remote.run(move |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        let id = ctx.spawn_entity("minecraft:zombie", [4.5, 65.0, 0.5]).expect("a zombie");
        assert!(ctx.give_entity_effect(id, effect, 1200, 0));
        *found.lock().unwrap() = Some(id);
    });
    drop(h.tick_all());
    let id: EntityId = zombie.lock().unwrap().expect("spawned");
    h.remote.run(move |ctx| assert!(ctx.kill_entity(id)));
    let sent: Vec<acacia_proto::RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    (h, sent)
}

#[test]
fn a_wind_charged_mob_bursts_as_it_dies_and_pushes_a_player_off() {
    let (_, sent) = killed_with(Effect::WindCharged);
    let burst = all::<LevelEvent>(&sent).iter().any(|event| event.event == LevelEventEvent::ParticleWindExplosion);
    let pushed = all::<SetEntityMotion>(&sent).iter().find(|motion| motion.runtime_entity_id == 1).map(|motion| motion.velocity.x);
    assert!(burst && pushed.is_some_and(|x| x < -0.2), "burst {burst}, pushed {pushed:?}");
}

#[test]
fn a_weaving_mob_leaves_two_or_three_cobwebs_about_it() {
    let (mut h, _) = killed_with(Effect::Weaving);
    let webs = Arc::new(Mutex::new(0));
    let counted = webs.clone();
    h.remote.run(move |ctx| {
        let cells = (3..=5).flat_map(|x| (65..=66).flat_map(move |y| (-1..=1).map(move |z| [x, y, z])));
        *counted.lock().unwrap() = cells.filter(|cell| ctx.block(*cell).is_some_and(|block| block.name == "minecraft:web")).count();
    });
    // A remote call runs in the shard's next tick.
    drop(h.tick_all());
    let webs = *webs.lock().unwrap();
    assert!((2..=3).contains(&webs), "{webs} cobwebs");
}

#[test]
fn an_oozing_mob_leaves_two_slimes_of_the_second_size() {
    let (_, sent) = killed_with(Effect::Oozing);
    let slimes = all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:slime").count();
    assert_eq!(slimes, 2);
}

#[test]
fn an_infested_cow_hit_lets_out_a_silverfish_now_and_then() {
    // A time in ten a hurt: forty hurts by instant damage, mended between, let one out but one
    // time in sixty.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let cow = Arc::new(Mutex::new(None));
    let found = cow.clone();
    h.remote.run(move |ctx| {
        let id = ctx.spawn_entity("minecraft:cow", [2.5, 65.0, 0.5]).expect("a cow");
        assert!(ctx.give_entity_effect(id, Effect::Infested, 6000, 0));
        *found.lock().unwrap() = Some(id);
    });
    drop(h.tick_all());
    let id: EntityId = cow.lock().unwrap().expect("spawned");
    let mut silverfish = 0;
    for _ in 0..40 {
        h.remote.run(move |ctx| assert!(ctx.give_entity_effect(id, Effect::InstantDamage, 1, 0)));
        for _ in 0..12 {
            silverfish += all::<AddEntity>(h.tick_all().to(1)).iter().filter(|added| added.entity_type == "minecraft:silverfish").count();
        }
        h.remote.run(move |ctx| assert!(ctx.give_entity_effect(id, Effect::InstantHealth, 1, 1)));
        drop(h.tick_all());
    }
    assert!(silverfish >= 1, "no silverfish in forty hurts");
}
