//! A loosed arrow turns as it flies, as BDS's did (research/captures/bds-shoot.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, MoveEntityDelta};
use common::{Harness, all};
use sapling_api::{Block, Difficulty, GameMode, PlayerId};

#[test]
fn a_skeletons_arrow_is_told_pitching_down_as_it_flies() {
    // Shot down from a pillar, the arrow falls ever steeper, a pitch byte a move or so.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.receive(2, &common::input_at([0.5, 65.0, -6.5]));
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        ctx.set_game_mode(PlayerId(2), GameMode::Creative);
        (64..74).for_each(|y| assert!(ctx.set_block([0, y, 6], &Block::new("stone"))));
        assert!(ctx.spawn_entity("minecraft:skeleton", [0.5, 74.0, 6.5]).is_some());
    });
    let sent: Vec<RawPacket> = (0..240).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    // Every arrow told turning at least twice; one that met something at once is never turned.
    let arrows = all::<AddEntity>(&sent).into_iter().filter(|added| added.entity_type == "minecraft:arrow");
    let turned: Vec<(f32, Vec<i8>)> = arrows
        .map(|arrow| (arrow.pitch, all::<MoveEntityDelta>(&sent).iter().filter(|moved| moved.runtime_entity_id == arrow.runtime_id).filter_map(|moved| moved.rot_x).map(|byte| byte as i8).collect::<Vec<_>>()))
        .filter(|(_, pitches)| pitches.len() >= 2)
        .collect();
    assert!(!turned.is_empty(), "no arrow was told turning twice");
    for (announced, pitches) in turned {
        assert!(pitches.windows(2).all(|pair| pair[1] < pair[0]) && (pitches[0] as f32) * 1.40625 < announced, "announced at {announced}, then {pitches:?}");
    }
}
