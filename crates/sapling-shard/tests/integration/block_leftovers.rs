//! Block leftovers of the parity round (docs/research/blocks.md): beds bursting outside the
//! Overworld.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{
    LevelEvent, LevelEventEvent, PositionTrackingDbBroadcast, PositionTrackingDbBroadcastBroadcastAction, PositionTrackingDbRequest, PositionTrackingDbRequestAction,
};
use common::asking::ask;
use acacia_proto::types::Action as BlockAction;
use common::{Harness, all, block_actions, click, shooting};
use sapling_api::{Block, GameMode, Item, PlayerId, RuleValue, Slot};
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

fn silverfish(h: &mut Harness) -> Vec<[f32; 3]> {
    ask(h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:silverfish").map(|entity| entity.feet).collect())
}

#[test]
fn infested_stone_broken_by_hand_or_blasted_lets_a_silverfish_out() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.set_difficulty(sapling_api::Difficulty::Easy));
    h.remote.run(|ctx| assert!(ctx.set_block([1, 64, 0], &Block::new("minecraft:infested_stone")) && ctx.set_block([-6, 64, -6], &Block::new("minecraft:infested_cobblestone"))));
    h.tick_all();
    h.receive(1, &block_actions(&[(BlockAction::StartBreak, [1, 64, 0])]));
    (0..120).for_each(|_| drop(h.tick_all()));
    h.receive(1, &block_actions(&[(BlockAction::PredictBreak, [1, 64, 0])]));
    h.tick_all();
    let out = silverfish(&mut h);
    assert!(out.len() == 1 && (out[0][0] - 1.0).abs() < 0.3 && (out[0][2] - 0.0).abs() < 0.3, "{out:?}");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:tnt", [-5.5, 65.0, -5.5]).is_some()));
    (0..90).for_each(|_| drop(h.tick_all()));
    assert_eq!(silverfish(&mut h).len(), 2);
}

/// The dust's signal, after the circuit settles.
fn dust(h: &mut Harness) -> String {
    (0..6).for_each(|_| drop(h.tick_all()));
    ask(h, |ctx| ctx.block([4, 65, 0]).map(|dust| dust.properties).unwrap_or_default())
}

#[test]
fn a_comparator_reads_a_frame_behind_its_block_by_turn_and_a_framed_map_shows() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:diamond", 1))));
        assert!(ctx.update_block([2, 65, 0], &Block::new("minecraft:stone")));
        assert!(ctx.update_block([1, 65, 0], &Block::new("minecraft:frame").with("facing_direction=4,item_frame_map_bit=0,item_frame_photo_bit=0")));
        assert!(ctx.update_block([3, 65, 0], &Block { name: "minecraft:unpowered_comparator".into(), properties: "minecraft:cardinal_direction=west".into() }));
        assert!(ctx.update_block([4, 65, 0], &Block::new("minecraft:redstone_wire")));
        assert!(ctx.set_block([1, 65, 3], &Block::new("minecraft:frame").with("facing_direction=1,item_frame_map_bit=0,item_frame_photo_bit=0")));
    });
    assert_eq!(dust(&mut h), "redstone_signal=0");
    h.receive(1, &click([1, 65, 0], 4, None));
    assert_eq!(dust(&mut h), "redstone_signal=1");
    h.receive(1, &click([1, 65, 0], 4, None));
    assert_eq!(dust(&mut h), "redstone_signal=2");

    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:filled_map", 1)))));
    h.tick_all();
    h.receive(1, &click([1, 65, 3], 1, None));
    h.tick_all();
    let shown = ask(&mut h, |ctx| ctx.block([1, 65, 3]).map(|frame| frame.properties)).unwrap_or_default();
    assert!(shown.contains("item_frame_map_bit=1"), "{shown}");
}

/// What the shard answers Alex's request for the target of `handle`.
fn tracked(h: &mut Harness, handle: i32) -> (PositionTrackingDbBroadcastBroadcastAction, String) {
    h.receive(1, &PositionTrackingDbRequest { action: PositionTrackingDbRequestAction::Query, tracking_id: handle });
    let told = all::<PositionTrackingDbBroadcast>(h.tick_all().to(1));
    let [told] = told.try_into().expect("one answer");
    assert_eq!(told.tracking_id, handle);
    (told.broadcast_action, format!("{:?}", told.nbt.decode().unwrap().value))
}

#[test]
fn a_compass_clicked_on_a_lodestone_points_at_it_until_it_goes() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:compass", 2))));
        assert!(ctx.set_block([2, 65, 0], &Block::new("minecraft:lodestone")));
    });
    h.tick_all();
    h.receive(1, &click([2, 65, 0], 1, None));
    h.tick_all();
    let held = ask(&mut h, |ctx| [Slot::Main(0), Slot::Main(1)].map(|slot| ctx.item(ALEX, slot).map(|item| (item.name, item.count))));
    assert_eq!(held, [Some(("minecraft:compass".into(), 1)), Some(("minecraft:lodestone_compass".into(), 1))]);
    let (told, record) = tracked(&mut h, 1);
    assert!(told == PositionTrackingDbBroadcastBroadcastAction::Update && record.contains("Int(2), Int(65), Int(0)") && record.contains("0x00000001"), "{told:?} {record}");
    assert_eq!(tracked(&mut h, 7).0, PositionTrackingDbBroadcastBroadcastAction::NotFound);
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 0], &Block::air())));
    h.tick_all();
    assert_eq!(tracked(&mut h, 1).0, PositionTrackingDbBroadcastBroadcastAction::Destory);
}

fn tilt_at(h: &mut Harness, pos: [i32; 3]) -> String {
    let properties = ask(h, move |ctx| ctx.block(pos).map(|leaf| leaf.properties)).unwrap_or_default();
    properties.split(',').find_map(|pair| pair.strip_prefix("big_dripleaf_tilt=")).unwrap_or_default().to_owned()
}

#[test]
fn a_big_dripleaf_tilts_under_a_player_and_comes_back_up() {
    const LEAF: [i32; 3] = [2, 65, 0];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_block(LEAF, &Block::new("minecraft:big_dripleaf").with("big_dripleaf_head=1,big_dripleaf_tilt=none,minecraft:cardinal_direction=south"))));
    h.tick_all();
    shooting::face(&mut h, 1, ALEX, [2.5, 65.9375, 0.5], 0.0);
    let mut seen = vec![tilt_at(&mut h, LEAF)];
    (0..10).for_each(|_| drop(h.tick_all()));
    seen.push(tilt_at(&mut h, LEAF));
    (0..10).for_each(|_| drop(h.tick_all()));
    seen.push(tilt_at(&mut h, LEAF));
    shooting::face(&mut h, 1, ALEX, [-4.5, 65.0, -4.5], 0.0);
    (0..100).for_each(|_| drop(h.tick_all()));
    seen.push(tilt_at(&mut h, LEAF));
    assert_eq!(seen, ["unstable", "partial_tilt", "full_tilt", "none"]);
}

#[test]
fn an_arrow_tilts_a_big_dripleaf_fully() {
    const LEAF: [i32; 3] = [0, 65, 0];
    let mut h = shooting::ready(GameMode::Survival, &[("minecraft:bow", 1, 0), ("minecraft:arrow", 4, 0)], 90.0);
    h.remote.run(|ctx| assert!(ctx.set_block(LEAF, &Block::new("minecraft:big_dripleaf").with("big_dripleaf_head=1,big_dripleaf_tilt=none,minecraft:cardinal_direction=south"))));
    h.tick_all();
    shooting::face(&mut h, 1, ALEX, [0.5, 69.0, 0.5], 90.0);
    shooting::draw(&mut h, 20, 10);
    assert_eq!(tilt_at(&mut h, LEAF), "full_tilt");
}

#[test]
fn a_bed_in_the_nether_does_nothing_with_respawn_blocks_explode_off() {
    let mut h = by_a_nether_bed(false);
    h.receive(1, &click(FOOT, 1, None));
    let sent: Vec<RawPacket> = h.tick_all().to(1).to_vec();
    assert!(all::<LevelEvent>(&sent).iter().all(|event| event.event != LevelEventEvent::ParticleExplosion));
    assert_eq!(block_in_nether(&mut h, HEAD), "minecraft:bed");
}
