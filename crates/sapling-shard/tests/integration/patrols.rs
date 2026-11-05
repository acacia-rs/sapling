//! Pillager patrols: the captain wanders off, its followers keep up (docs/research/structure-spawns.md "Patrols").

use crate::common;

use acacia_proto::nbt::{List, Value};
use common::Harness;
use sapling_api::{Difficulty, EntityId, GameMode, PlayerId};
use sapling_world::{ChunkPos, World, WorldStorage};

const ALEX: PlayerId = PlayerId(1);

fn feet(h: &mut Harness, id: EntityId) -> [f32; 3] {
    common::asking::ask(h, move |ctx| ctx.entities().into_iter().find(|entity| entity.id == id).expect("alive").feet)
}

fn flat(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[test]
fn a_patrol_captain_wanders_and_its_followers_keep_up() {
    let mut level = World::void();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.fill([-64, 64, -64], [63, 64, 63], stone);
    let mut h = Harness::with(common::config(), level);
    h.enter(1, "Alex");
    common::asking::ask(&mut h, |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_difficulty(Difficulty::Normal);
    });
    let spawned = common::asking::ask(&mut h, |ctx| {
        let captain = ctx.summon_as(None, "minecraft:pillager", [20.5, 65.0, 20.5], 0.0, Some("minecraft:promote_to_patrol_captain"));
        let followers = [[30.5, 65.0, 20.5], [20.5, 65.0, 30.5]].map(|at| ctx.summon_as(None, "minecraft:pillager", at, 0.0, Some("minecraft:spawn_as_patrol_follower")));
        (captain, followers)
    });
    let (Some(captain), [Some(first), Some(second)]) = spawned else { panic!("summoned: {spawned:?}") };
    let start = feet(&mut h, captain);
    (0..200).for_each(|_| drop(h.tick_all()));
    let now = feet(&mut h, captain);
    assert!(flat(start, now) > 3.0, "the captain makes for its random block: {start:?} to {now:?}");
    for follower in [first, second] {
        let at = feet(&mut h, follower);
        assert!(flat(at, now) < 8.0, "a follower keeps by its captain: {at:?} against {now:?}");
    }
}

#[test]
fn a_kept_patrol_captain_walks_on_and_keeps_its_group() {
    let groups = Value::List(List { tag: 8, items: vec![Value::String("+patrol_captain".into())] });
    let storage = common::stored::floor_with(common::stored::actor_with(-5, "minecraft:pillager", [8.5, 64.0, 8.5], vec![("definitions", groups)]));
    let config = sapling_shard::ShardConfig { difficulty: Difficulty::Normal, ..common::config() };
    let mut h = Harness::with(config, World::with_storage(storage.clone()));
    common::stored::enter_stored(&mut h, 1);
    common::asking::ask(&mut h, |ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    let captain = common::asking::ask(&mut h, |ctx| ctx.entities().into_iter().find(|entity| entity.kind == "minecraft:pillager").expect("loaded").id);
    let start = feet(&mut h, captain);
    (0..100).for_each(|_| drop(h.tick_all()));
    let now = feet(&mut h, captain);
    assert!(flat(start, now) > 2.0, "a kept captain walks on, mobs not strolling: {start:?} to {now:?}");
    h.shard.save_worlds();
    let stored = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("saved");
    let kept = sapling_world::stored_actors(&stored.actors).unwrap();
    assert!(kept.iter().any(|(_, _, compound)| sapling_world::actor_groups(compound).contains(&"patrol_captain".to_owned())), "{kept:?}");
}
