//! Natural mobs going (docs/research/mob-spawning.md "Despawning"), on spawning.rs's dark floor.

use crate::common;
use crate::spawning::{ALEX, ask, at_midnight_with, monsters, run};

use common::Harness;
use sapling_api::{RuleValue, ShardCtx};

/// `/away`: the sender 120 east, moved by a command as the drill's `/tp` moved it.
#[derive(sapling_api::Command)] #[command(crate = "sapling_api")]
struct Away;
impl sapling_api::Command for Away {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &sapling_api::Sender, _: &mut sapling_api::Output) { ctx.teleport(sender.player().expect("a player").id, [120.5, 65.0, 0.5]) }
}

/// bds-w3despawn.txt: mobs out of every tick region stay however long (`DespawnSystem` views only
/// `ActorTickedComponent`); back near them, the natural ones past 44 go. 12 chunks keep their columns
/// loaded 120 east; on the command's tick Alex's view still covers them.
#[test]
fn natural_mobs_out_of_every_tick_region_stay_and_go_once_ticked_again() {
    let mut h = at_midnight_with(sapling_shard::ShardConfig { max_view_radius: 12, ..common::config() }, "minecraft:stone");
    h.shard.add_command::<Away>();
    h.receive(1, &acacia_proto::packets::RequestChunkRadius { chunk_radius: 12, max_radius: 12 });
    run(&mut h, 400);
    let ids = |h: &mut Harness| monsters(h).into_iter().map(|monster| monster.id).collect::<Vec<_>>();
    let natural = ids(&mut h);
    let summoned = ask(&mut h, |ctx| ctx.spawn_entity("minecraft:zombie", [10.5, 65.0, 10.5])).expect("a zombie");
    let origin = acacia_proto::types::CommandOrigin { r#type: "player".into(), uuid: acacia_proto::manual::Uuid([1; 16]), request_id: "req".into(), player_entity_id: 0 };
    h.receive(1, &acacia_proto::packets::CommandRequest { command: "/away".into(), origin, internal: false, version: "latest".into() });
    run(&mut h, 1200);
    assert_eq!(ask(&mut h, |ctx| ctx.position(ALEX).map(|at| at.feet[0])), Some(120.5), "moved by the command");
    let left = ids(&mut h);
    assert!(natural.iter().chain([&summoned]).all(|id| left.contains(id)), "{natural:?} and {summoned:?}, then {left:?}");
    h.remote.run(|ctx| ctx.teleport(ALEX, [0.5, 135.0, 0.5]));
    run(&mut h, 3);
    let left = ids(&mut h);
    assert!(left.contains(&summoned) && !natural.is_empty() && natural.iter().all(|id| !left.contains(id)), "{natural:?}, then {left:?}");
}

/// BDS caps the far distance at 44 only where the chunk tick range is 4 or less (`_canDespawn`,
/// docs/research/mob-spawning.md "Despawning"): 70 blocks over them, its monsters go at 4 and stay at 6.
#[test]
fn natural_mobs_70_off_go_at_tick_distance_4_and_stay_at_6() {
    for (tick_distance, stay) in [(4, false), (6, true)] {
        let mut h = at_midnight_with(sapling_shard::ShardConfig { tick_distance, ..common::config() }, "minecraft:stone");
        run(&mut h, 400);
        let before: Vec<_> = monsters(&mut h).into_iter().filter(|monster| monster.feet[0].hypot(monster.feet[2]) < 46.0).map(|monster| monster.id).collect();
        assert!(!before.is_empty());
        h.remote.run(|ctx| ctx.teleport(ALEX, [0.5, 135.0, 0.5]));
        run(&mut h, 3);
        let after: Vec<_> = monsters(&mut h).into_iter().map(|monster| monster.id).collect();
        assert_eq!(before.iter().all(|id| after.contains(id)), stay, "tick distance {tick_distance}: {before:?} then {after:?}");
        assert!(stay || before.iter().all(|id| !after.contains(id)), "tick distance {tick_distance}");
    }
}

/// bds-aw3despawn.txt: natural mobs between their near and far distance go only once idle 30 s, then by
/// the 1 in 800 roll each tick. At tick distance 6 (no 44 cap) 38 over the floor, every monster is past 32.
#[test]
fn natural_mobs_past_32_stay_while_idle_under_30_s_then_go_by_the_roll() {
    let mut h = at_midnight_with(sapling_shard::ShardConfig { tick_distance: 6, ..common::config() }, "minecraft:stone");
    run(&mut h, 400);
    h.remote.run(|ctx| {
        assert!(ctx.set_game_rule("doMobSpawning", RuleValue::Bool(false)));
        assert!(ctx.set_game_rule("doDaylightCycle", RuleValue::Bool(false)));
        ctx.teleport(ALEX, [0.5, 103.0, 0.5]);
    });
    run(&mut h, 3);
    let before: Vec<_> = monsters(&mut h).into_iter().map(|monster| monster.id).collect();
    assert!(!before.is_empty());
    run(&mut h, 150);
    let after: Vec<_> = monsters(&mut h).into_iter().map(|monster| monster.id).collect();
    assert!(before.iter().all(|id| after.contains(id)), "{before:?} then {after:?}");
    run(&mut h, 8000);
    assert_eq!(monsters(&mut h), []);
}
