//! Pillager patrols: the captain wanders off, its followers keep up (docs/research/structure-spawns.md "Patrols").

use crate::common;

use common::Harness;
use sapling_api::{Difficulty, EntityId, GameMode, PlayerId};
use sapling_world::World;

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
