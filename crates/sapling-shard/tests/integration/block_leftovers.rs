//! Block leftovers of the parity round (docs/research/blocks.md): beds bursting outside the
//! Overworld.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use common::asking::ask;
use common::{Harness, all, click};
use sapling_api::{Block, PlayerId, RuleValue};
use sapling_world::{NETHER, World};

const ALEX: PlayerId = PlayerId(1);
const FOOT: [i32; 3] = [2, 70, 0];
const HEAD: [i32; 3] = [2, 70, 1];
const NETHER_NAME: &str = "default:nether";

/// A void Nether with a netherrack floor at y 69 around the origin.
fn nether() -> World {
    let mut nether = World::void_in(NETHER);
    let netherrack = nether.block_id("minecraft:netherrack").unwrap();
    nether.fill([-8, 69, -8], [7, 69, 7], netherrack);
    nether
}

/// Alex in the Nether beside a bed, its head to the south.
fn by_a_nether_bed(bursts: bool) -> Harness {
    let mut h = Harness::new();
    assert!(h.shard.add_dimension("default", nether()));
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, NETHER, [0.5, 70.0, 0.5])));
    (0..5).for_each(|_| drop(h.tick_all()));
    h.remote.run(move |ctx| {
        let placed = ctx.within(NETHER_NAME, |ctx| {
            let half = |head: u8| Block { name: "minecraft:bed".into(), properties: format!("direction=0,head_piece_bit={head},occupied_bit=0") };
            ctx.set_block(FOOT, &half(0)) && ctx.set_block(HEAD, &half(1)) && ctx.set_game_rule("respawnBlocksExplode", RuleValue::Bool(bursts))
        });
        assert_eq!(placed, Some(true));
    });
    h.tick_all();
    h
}

fn block_in_nether(h: &mut Harness, pos: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.within(NETHER_NAME, |ctx| ctx.block(pos)).flatten().map(|block| block.name)).expect("a block")
}

#[test]
fn a_bed_bursts_in_the_nether_between_its_halves() {
    let mut h = by_a_nether_bed(true);
    h.receive(1, &click(FOOT, 1, None));
    let sent: Vec<RawPacket> = h.tick_all().to(1).to_vec();
    let flash = all::<LevelEvent>(&sent).into_iter().find(|event| event.event == LevelEventEvent::ParticleExplosion).expect("a blast");
    assert_eq!((flash.data, flash.position.x, flash.position.y, flash.position.z), (5, 2.5, 70.5, 1.0));
    for half in [FOOT, HEAD] {
        let left = block_in_nether(&mut h, half);
        assert!(left == "minecraft:air" || left == "minecraft:fire", "{half:?}: {left}");
    }
}

#[test]
fn a_bed_in_the_nether_does_nothing_with_respawn_blocks_explode_off() {
    let mut h = by_a_nether_bed(false);
    h.receive(1, &click(FOOT, 1, None));
    let sent: Vec<RawPacket> = h.tick_all().to(1).to_vec();
    assert!(all::<LevelEvent>(&sent).iter().all(|event| event.event != LevelEventEvent::ParticleExplosion));
    assert_eq!(block_in_nether(&mut h, HEAD), "minecraft:bed");
}
