//! Group AP of the fourteenth wave: a mob a player hit flees away from it (bds-ap2flee).

use crate::common;

use common::asking::ask;
use sapling_api::{EntityId, GameMode, PlayerId};

/// bds-ap2flee.txt: six cows hit from the west or the north each ran 9 to 13 blocks off in five seconds
/// and never came nearer the actor than they stood: `PanicGoal` draws its places by
/// `RandomPos::getPosAvoid`, which turns about a try across toward the attacker.
#[test]
fn a_cow_hit_by_a_player_flees_away_from_it() {
    let mut h = common::Harness::with(sapling_shard::ShardConfig { mobs_stroll: true, ..common::config() }, common::platform());
    h.enter(1, "Alex");
    for (at, from) in [([2.5, 65.0, 0.5], [0.5, 65.0, 0.5]), ([0.5, 65.0, 2.5], [0.5, 65.0, 0.5]), ([-1.5, 65.0, 0.5], [0.5, 65.0, 0.5])] {
        let cow = ask(&mut h, move |ctx| {
            ctx.set_game_mode(PlayerId(1), GameMode::Survival);
            ctx.teleport(PlayerId(1), from);
            ctx.spawn_entity("minecraft:cow", at).expect("a cow").0
        });
        h.receive(1, &common::attack_from(cow, from));
        let mut off = Vec::new();
        for _ in 0..70 {
            let feet = ask(&mut h, move |ctx| ctx.entity(EntityId(cow)).expect("still there").feet);
            off.push((feet[0] - from[0]).hypot(feet[2] - from[2]));
        }
        let least = off.iter().copied().fold(f32::MAX, f32::min);
        assert!(least >= 1.5 && off[69] > 4.0, "from {at:?}: least {least}, last {}", off[69]);
        ask(&mut h, move |ctx| assert!(ctx.remove_entity(EntityId(cow))));
    }
}
