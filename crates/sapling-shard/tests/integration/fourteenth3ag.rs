//! Group AG of the fourteenth parity brief: what a piston tells of a body it carries off its block
//! (research/captures/bds-u3carry.txt), and a client's own close answered between ticks
//! (bds-u3chest.txt).

use crate::common;
use crate::common::requests::request;
use crate::riding::spawned;

use acacia_proto::RawPacket;
use acacia_proto::packets::{BlockEvent, ContainerClose, ContainerOpen, MoveEntityDelta};
use acacia_proto::types::WindowType;
use common::{Harness, all, click};
use sapling_api::{Block, PlayerId};
use sapling_net::{ConnId, Outbound};

/// What Alex is sent between ticks.
fn sent_between_ticks(h: &mut Harness) -> Vec<RawPacket> {
    h.shard.between_ticks();
    let mut sent = Vec::new();
    while let Ok(command) = h.net.sent.try_recv() {
        if let Outbound::Send { conn, packets } = command
            && conn == ConnId::new(1)
        {
            sent.extend(packets.into_iter().map(|packet| RawPacket::parse(packet).unwrap()));
        }
    }
    sent
}

#[test]
fn a_stand_pinned_flush_is_told_by_the_arms_finish_with_no_move() {
    // bds-ag3pin.txt: x 3.75 flagged as the arm starts, then a flagged move of no axes as it finishes.
    let mut h = common::Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for (pos, name, properties) in [([0, 65, 4], "piston", "facing_direction=4"), ([2, 65, 4], "obsidian", "")] {
            assert!(ctx.set_block(pos, &Block::new(name).with(properties)));
        }
    });
    h.tick_all();
    let stand = spawned(&mut h, &[("minecraft:armor_stand", [1.5, 65.0, 4.1])])[0];
    (0..3).for_each(|_| _ = h.tick_all());
    h.remote.run(|ctx| assert!(ctx.set_block([0, 66, 4], &Block::new("redstone_block"))));
    let flagged: Vec<[Option<f32>; 3]> = (0..6).flat_map(|_| all::<MoveEntityDelta>(h.tick_all().to(1))).filter(|moved| moved.runtime_entity_id == stand.0 && moved.force_move_local_entity).map(|moved| [moved.x, moved.y, moved.z]).collect();
    assert_eq!(flagged, [[Some(1.75), None, None], [None, None, None]]);
}

#[test]
fn a_client_closing_a_chest_is_answered_between_ticks_unless_it_sent_more_first() {
    // bds-u3chest.txt: BDS answered the actor's own ContainerClose 7 ms on, before its tick's block updates.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_block([1, 65, 0], &Block::new("chest").with("minecraft:cardinal_direction=north"))));
    h.tick_all();
    fn open(h: &mut Harness) -> ContainerClose {
        h.receive(1, &click([1, 65, 0], 1, None));
        let window_id = all::<ContainerOpen>(h.tick_all().to(1))[0].window_id;
        (0..2).for_each(|_| _ = h.tick_all());
        ContainerClose { window_id, window_type: WindowType::Container, server: false }
    }
    let close = open(&mut h);
    h.receive(1, &close);
    let answered = sent_between_ticks(&mut h);
    assert_eq!(all::<ContainerClose>(&answered).iter().map(|close| close.server).collect::<Vec<_>>(), [false]);
    assert!(all::<BlockEvent>(&answered).iter().any(|lid| lid.data == 0), "the lid shuts with it");
    assert!(all::<ContainerClose>(h.tick_all().to(1)).is_empty(), "answered once");
    let close = open(&mut h);
    h.receive(1, &request(-3, Vec::new()));
    h.receive(1, &close);
    assert!(all::<ContainerClose>(&sent_between_ticks(&mut h)).is_empty(), "a request sent first goes first, in the tick");
    assert_eq!(all::<ContainerClose>(h.tick_all().to(1)).len(), 1);
}

#[test]
fn a_body_carried_off_a_block_is_told_the_pistons_half_then_its_own_fall_and_paced_after() {
    // bds-u3carry.txt, the actor ~12 off (a tick between moves): 4.0 flagged and on the ground, 4.5
    // falling unflagged in the start tick; 5.25 flagged with y held in the next, its fall untold.
    let mut h = common::Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.teleport(PlayerId(1), [-7.5, 65.0, 0.5]);
        for (pos, name, properties) in [([3, 65, 0], "piston", "facing_direction=4"), ([4, 65, 0], "stone", "")] {
            assert!(ctx.set_block(pos, &Block::new(name).with(properties)));
        }
    });
    (0..3).for_each(|_| _ = h.tick_all());
    let stand = spawned(&mut h, &[("minecraft:armor_stand", [4.5, 66.0, 0.5])])[0];
    (0..3).for_each(|_| _ = h.tick_all());
    h.remote.run(|ctx| assert!(ctx.set_block([3, 66, 0], &Block::new("redstone_block"))));
    let ticks: Vec<Vec<MoveEntityDelta>> = (0..6).map(|_| all::<MoveEntityDelta>(h.tick_all().to(1)).into_iter().filter(|moved| moved.runtime_entity_id == stand.0).collect()).collect();
    let told: Vec<Vec<(bool, Option<f32>, bool, bool)>> = ticks.iter().filter(|moves| !moves.is_empty()).map(|moves| moves.iter().map(|moved| (moved.force_move_local_entity, moved.x, moved.y.is_some(), moved.on_ground)).collect()).collect();
    assert_eq!(told[0], [(true, Some(5.0), false, true), (false, Some(5.5), true, false)], "{ticks:?}");
    assert_eq!(told[1], [(true, Some(6.25), false, false)], "{ticks:?}");
    assert_eq!(told[2], [(false, None, true, false)], "{ticks:?}");
}
