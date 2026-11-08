//! Group AT (research/parity/THIRTEENTH-BRIEF.md): slimes in a flat world.

use crate::common;

use common::Harness;
use common::asking::ask;
use sapling_api::{Difficulty, EntityId, GameMode, PlayerId};
use sapling_world::World;

const ALEX: PlayerId = PlayerId(1);

/// The slimes that appear in daylight on grass at y -61 round Alex in 40 rounds of 100 ticks, each
/// round's taken away so that no cap holds them back.
fn slimes_spawned(flat: bool) -> usize {
    let mut world = World::void();
    let grass = world.block_id("minecraft:grass_block").unwrap();
    world.fill([-64, -61, -64], [63, -61, 63], grass);
    world.level_mut().flat = flat;
    let mut h = Harness::with(common::config(), world);
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_time(1000);
        ctx.teleport(ALEX, [0.5, -60.0, 0.5]);
    });
    h.tick_all();
    let mut spawned = 0;
    for _ in 0..40 {
        (0..100).for_each(|_| drop(h.tick_all()));
        let slimes: Vec<EntityId> = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:slime").map(|entity| entity.id).collect());
        spawned += slimes.len();
        h.remote.run(move |ctx| slimes.iter().for_each(|&id| assert!(ctx.remove_entity(id))));
    }
    spawned
}

/// bds-an1grow.txt: BDS's flat world spawned 20 slimes in 584 s where sapling's spawned 45; a slime's
/// own check (142999ca0) passes in a flat world only on `nextInt(4) == 1` (mob-spawning.md "Slimes").
#[test]
fn a_flat_world_spawns_a_quarter_of_the_slimes() {
    let (plain, flat) = (slimes_spawned(false), slimes_spawned(true));
    assert!(flat > 0 && flat * 2 < plain, "flat {flat}, plain {plain}");
}
