//! A world's Nether: a dimension change as BDS makes one (research/captures/bds-dims-travel.txt)
//! and what the dimensions of a world share.

use crate::common;

use acacia_proto::Packet;
use acacia_proto::packets::{AddEntity, AddPlayer, ChangeDimension, CorrectPlayerMovePrediction, LevelChunk, LevelEventGeneric, PlayerAction, RemoveEntity};
use acacia_proto::types::{Action, BlockCoordinates};
use common::{Harness, all, ids, input_at};
use sapling_api::{PlayerId, RuleValue};
use sapling_world::{NETHER, World};

const ALEX: PlayerId = PlayerId(1);

/// A void Nether with a netherrack floor at y 69 around the origin.
fn nether() -> World {
    let mut nether = World::void_in(NETHER);
    let netherrack = nether.block_id("minecraft:netherrack").unwrap();
    nether.fill([-8, 69, -8], [7, 69, 7], netherrack);
    nether
}

/// Alex (1) and Steve (2) on the platform of a world with a Nether.
fn two_players() -> Harness {
    let mut h = Harness::new();
    assert!(h.shard.add_dimension("default", nether()));
    assert!(!h.shard.add_dimension("default", nether()), "a world has one Nether");
    assert!(!h.shard.add_dimension("default", World::void()), "an overworld is no other dimension");
    assert!(!h.shard.add_dimension("elsewhere", nether()));
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.tick_all();
    h
}

fn ack() -> PlayerAction {
    let origin = BlockCoordinates { x: 0, y: 0, z: 0 };
    PlayerAction { runtime_entity_id: 1, action: Action::DimensionChangeAck, position: origin.clone(), result_position: origin, face: 0 }
}

/// Ticks until Alex is told it may leave its loading screen; what Alex was sent meanwhile.
fn until_ready(h: &mut Harness) -> Vec<acacia_proto::RawPacket> {
    let mut sent = Vec::new();
    for _ in 0..10 {
        sent.extend(h.tick_all().to(1).iter().cloned());
        if sent.iter().any(|p| p.id == PlayerAction::ID) {
            return sent;
        }
    }
    panic!("never ready: {:?}", ids(&sent));
}

#[test]
fn a_move_into_the_nether_and_back_goes_as_on_bds() {
    let mut h = two_players();
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, NETHER, [0.5, 70.0, 0.5])));
    let sent = h.tick_all();
    // What it saw goes first, then the change with its first loading screen, then the sleepers.
    assert_eq!(ids(sent.to(1)), [RemoveEntity::ID, ChangeDimension::ID, LevelEventGeneric::ID]);
    let change = all::<ChangeDimension>(sent.to(1)).remove(0);
    assert_eq!((change.dimension, [change.position.x, change.position.y, change.position.z], change.respawn, change.loading_screen_id), (1, [0.5, 70.0, 0.5], false, Some(0)));
    assert_eq!(all::<RemoveEntity>(sent.to(2)), [RemoveEntity { entity_id_self: 1 }], "gone for the overworld");
    let sleepers = all::<LevelEventGeneric>(sent.to(2)).remove(0);
    assert_eq!(sleepers.nbt.iter().find(|tag| tag.name == "overworldPlayerCount").map(|tag| format!("{:?}", tag.value)), Some("Int(1)".into()));

    // The Nether's columns, then the word to leave the loading screen; nothing of the overworld.
    h.receive(1, &input_at([30.0, 10.0, 30.0]));
    let sent = until_ready(&mut h);
    let columns = all::<LevelChunk>(&sent);
    assert!(columns.iter().all(|column| column.dimension == NETHER) && columns.iter().any(|column| column.highest_subchunk_count == Some(5)));
    assert_eq!(all::<PlayerAction>(&sent), [ack()], "BDS's own DimensionChangeAck");
    h.remote.run(|ctx| assert_eq!((ctx.position(ALEX).unwrap().feet, ctx.world_of(ALEX).as_deref(), ctx.dimension_of(ALEX)), ([0.5, 70.0, 0.5], Some("default:nether"), Some(1)), "inputs from the loading screen are not taken"));
    h.tick_all();

    // The client says it arrived: it is put where the server holds it, and moves from there.
    h.receive(1, &ack());
    let correction = all::<CorrectPlayerMovePrediction>(h.tick_all().to(1)).remove(0);
    assert_eq!([correction.position.x, correction.position.z, correction.position.y - sapling_data::EYE_HEIGHT], [0.5, 0.5, 70.0]);

    // Back, with the next loading screen; the overworld sees Alex again.
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, 0, [0.5, 65.0, 0.5])));
    let sent = h.tick_all();
    assert_eq!(all::<ChangeDimension>(sent.to(1)).remove(0).loading_screen_id, Some(1));
    assert_eq!(all::<AddPlayer>(sent.to(2)).len(), 1);
    h.remote.run(|ctx| assert_eq!(ctx.world_of(ALEX).as_deref(), Some("default")));
    h.tick_all();
}

#[test]
fn loading_screens_are_counted_over_every_player() {
    // research/captures/bds-w1screens.txt: the actor's trips 0 and 1, the watcher's 2 and 3.
    let mut h = two_players();
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, NETHER, [0.5, 70.0, 0.5])));
    let first = h.tick_all();
    h.remote.run(|ctx| assert!(ctx.travel(PlayerId(2), NETHER, [0.5, 70.0, 0.5])));
    let second = h.tick_all();
    assert_eq!([all::<ChangeDimension>(first.to(1)).remove(0).loading_screen_id, all::<ChangeDimension>(second.to(2)).remove(0).loading_screen_id], [Some(0), Some(1)]);
}

#[test]
fn a_dimension_shares_its_overworlds_time_and_rules() {
    let mut h = two_players();
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, NETHER, [0.5, 70.0, 0.5])));
    h.tick_all();
    h.remote.run(|ctx| {
        ctx.within("default:nether", |ctx| {
            ctx.set_time(1000);
            assert!(ctx.set_game_rule("doDaylightCycle", RuleValue::Bool(false)));
        });
        assert_eq!((ctx.time(), ctx.game_rule("dodaylightcycle").map(|(_, value)| value)), (1000, Some(RuleValue::Bool(false))), "set on the overworld");
        assert_eq!(ctx.within("default:nether", |ctx| ctx.time()), Some(1000));
    });
    h.tick_all();
    (0..5).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert_eq!(ctx.within("default:nether", |ctx| ctx.time()), Some(1000), "a stopped clock stops in both"));
    h.tick_all();
}

/// BDS's cow sent from the End by `/execute in overworld run tp` left the End's screens and was
/// announced in the overworld where it was sent (research/captures/bds-u1inherit.txt).
#[test]
fn a_mob_sent_into_another_dimension_leaves_one_screen_for_the_other() {
    let mut h = two_players();
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, NETHER, [0.5, 70.0, 0.5])));
    h.tick_all();
    h.receive(1, &input_at([0.5, 70.0, 0.5]));
    until_ready(&mut h);
    h.receive(1, &ack());
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:cow", [2.5, 65.0, 0.5]).is_some()));
    let sent: Vec<_> = (0..3).map(|_| h.tick_all()).collect();
    assert!(sent.iter().all(|sent| all::<AddEntity>(sent.to(1)).is_empty()), "none in Alex's Nether");
    let first = sent.iter().flat_map(|sent| all::<AddEntity>(sent.to(2))).next().expect("first in Steve's overworld");
    let (cow, unique) = (first.runtime_id, first.unique_id);
    h.remote.run(move |ctx| assert!(ctx.move_entity_into(sapling_api::EntityId(cow), NETHER, [1.5, 70.0, 1.5])));
    let sent = h.tick_all();
    assert_eq!(all::<RemoveEntity>(sent.to(2)), [RemoveEntity { entity_id_self: cow as i64 }]);
    let added = all::<AddEntity>(sent.to(1));
    // BDS gives it a new runtime id and keeps its unique id (bds-u1inherit.txt).
    let moved = added.iter().map(|added| added.runtime_id).find(|id| *id != cow).expect("a new runtime id");
    assert_eq!(added.iter().map(|added| (added.runtime_id, [added.position.x, added.position.y, added.position.z])).collect::<Vec<_>>(), [(moved, [1.5, 70.0, 1.5])]);
    assert_eq!(added.iter().map(|added| added.unique_id).collect::<Vec<_>>(), [unique]);
    h.remote.run(move |ctx| {
        assert!(ctx.entity(sapling_api::EntityId(cow)).is_none());
        assert_eq!(ctx.entity(sapling_api::EntityId(moved)).map(|cow| cow.feet[0]), Some(1.5));
        assert!(!ctx.move_entity_into(sapling_api::EntityId(moved), 2, [0.5, 70.0, 0.5]), "no End here");
    });
    h.tick_all();
}

#[test]
fn a_travel_needs_the_dimension() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(!ctx.travel(ALEX, NETHER, [0.5, 70.0, 0.5]), "no Nether here");
        assert!(ctx.travel(ALEX, 0, [3.5, 65.0, 0.5]), "its own dimension: a teleport");
        assert_eq!(ctx.dimension_of(ALEX), Some(0));
    });
    let sent = h.tick_all();
    assert!(all::<ChangeDimension>(sent.to(1)).is_empty() && !ids(sent.to(1)).is_empty());
}
