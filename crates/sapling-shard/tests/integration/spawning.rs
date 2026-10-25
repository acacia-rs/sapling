//! Natural spawning and despawning (docs/research/mob-spawning.md) and spawn eggs.

use crate::common;

use std::sync::{Arc, Mutex};

use common::{Harness, click};
use sapling_api::{Difficulty, EntityInfo, GameMode, Item, PlayerId, RuleValue, ShardCtx, Slot};
use sapling_world::World;

const ALEX: PlayerId = PlayerId(1);
const MIDNIGHT: i64 = 18000;
const MONSTERS: &[&str] = &["minecraft:zombie", "minecraft:zombie_villager_v2", "minecraft:skeleton", "minecraft:spider", "minecraft:creeper", "minecraft:enderman", "minecraft:witch"];

/// A void world with a floor of `floor` at y 64 over the four chunks each way of the origin: every
/// spawn's cap counts all of it.
fn floored(floor: &str) -> World {
    let mut level = World::void();
    let id = level.block_id(floor).unwrap();
    level.fill([-32, 64, -32], [31, 64, 31], id);
    level
}

/// Alex in creative, so nothing hunts it, at midnight on normal difficulty over `floor`.
fn at_midnight(floor: &str) -> Harness {
    let mut h = Harness::with(common::config(), floored(floor));
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_time(MIDNIGHT);
    });
    h.tick_all();
    h
}

fn ask<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> T {
    let found = Arc::new(Mutex::new(None));
    let out = found.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = Some(ask(ctx)));
    h.tick_all();
    found.lock().unwrap().take().expect("the handler ran")
}

fn monsters(h: &mut Harness) -> Vec<EntityInfo> {
    ask(h, |ctx| ctx.entities().into_iter().filter(|entity| MONSTERS.contains(&entity.kind.as_str())).collect())
}

fn run(h: &mut Harness, ticks: usize) {
    (0..ticks).for_each(|_| drop(h.tick_all()));
}

#[test]
fn monsters_appear_in_the_dark_away_from_the_player_up_to_their_cap() {
    let mut h = at_midnight("minecraft:stone");
    run(&mut h, 400);
    let found = monsters(&mut h);
    assert!(!found.is_empty(), "nothing spawned on a dark floor");
    for monster in &found {
        let across = (monster.feet[0] - 0.5).hypot(monster.feet[2] - 0.5);
        assert!((23.0..=45.0).contains(&across) && monster.feet[1] == 65.0, "{monster:?}");
    }
    run(&mut h, 1200);
    // The cap is checked before a herd of up to 4 comes.
    assert!(monsters(&mut h).len() <= 8 + 3, "{:?}", monsters(&mut h));
}

#[test]
fn a_lit_floor_spawns_nothing() {
    let mut h = at_midnight("minecraft:glowstone");
    run(&mut h, 200);
    assert_eq!(monsters(&mut h), []);
}

#[test]
fn peaceful_and_do_mob_spawning_off_spawn_nothing() {
    let mut h = at_midnight("minecraft:stone");
    h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Peaceful));
    run(&mut h, 200);
    assert_eq!(monsters(&mut h), []);
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        assert!(ctx.set_game_rule("doMobSpawning", RuleValue::Bool(false)));
    });
    run(&mut h, 200);
    assert_eq!(monsters(&mut h), []);
}

#[test]
fn natural_mobs_go_when_the_player_is_far_and_summoned_ones_stay() {
    let mut h = at_midnight("minecraft:stone");
    run(&mut h, 400);
    assert!(!monsters(&mut h).is_empty());
    let summoned = ask(&mut h, |ctx| ctx.spawn_entity("minecraft:zombie", [10.5, 65.0, 10.5])).expect("a zombie");
    h.remote.run(|ctx| ctx.teleport(ALEX, [1000.5, 65.0, 0.5]));
    run(&mut h, 3);
    let left: Vec<_> = monsters(&mut h).into_iter().map(|monster| monster.id).collect();
    assert_eq!(left, [summoned]);
}

#[test]
fn a_spawn_egg_hatches_on_the_face_clicked_and_is_spent_in_survival() {
    let mut h = Harness::with(common::config(), floored("minecraft:stone"));
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:pig_spawn_egg", 2)))));
    h.tick_all();
    h.receive(1, &click([2, 64, 0], 1, None));
    h.tick_all();
    let pigs = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:pig").map(|pig| pig.feet).collect::<Vec<_>>());
    assert_eq!(pigs, [[2.5, 65.0, 0.5]]);
    assert_eq!(ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|held| held.count)), Some(1));
}
