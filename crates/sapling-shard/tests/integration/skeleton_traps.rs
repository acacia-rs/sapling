//! A skeleton horse trap springs when a player of any mode comes within 10, as BDS's did
//! (research/captures/bds-u1trap.txt): four skeleton horses ridden by skeletons in iron helmets
//! where it stood, a bolt, and the trap gone; one farther off waits.

use crate::common;

use acacia_proto::packets::{AddEntity, SetEntityLink};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::{Difficulty, GameMode, PlayerId};

#[test]
fn a_trap_springs_four_riders_for_a_creative_player_within_ten() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
    });
    let far = ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:skeleton_horse", [20.5, 65.0, 0.5], 0.0, 0.0, Some("minecraft:set_trap"))).expect("summoned");
    let near = ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:skeleton_horse", [6.5, 65.0, 0.5], 0.0, 0.0, Some("minecraft:set_trap"))).expect("summoned");
    let sent: Vec<_> = (0..5).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let kinds = ask(&mut h, |ctx| ctx.entities().into_iter().map(|entity| (entity.id, entity.kind)).collect::<Vec<_>>());
    let count = |kind: &str| kinds.iter().filter(|(_, of)| of == kind).count();
    assert_eq!((count("minecraft:skeleton_horse"), count("minecraft:skeleton")), (5, 4), "{kinds:?}");
    assert!(kinds.iter().any(|(id, _)| *id == far), "the far trap waits");
    assert!(!kinds.iter().any(|(id, _)| *id == near), "the sprung trap is gone");
    assert!(all::<AddEntity>(&sent).iter().any(|added| added.entity_type == "minecraft:lightning_bolt"), "a bolt");
    assert_eq!(all::<SetEntityLink>(&sent).iter().filter(|link| link.link.r#type == 1).map(|link| link.link.rider_entity_id).collect::<std::collections::HashSet<_>>().len(), 4, "each skeleton rides");
}
