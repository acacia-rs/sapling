//! Nether portals: lit by fire in a frame, gone with the frame, and a trip through one that
//! makes a portal at the other end (research/captures/bds-portal-*.txt).

mod common;

use acacia_proto::Packet;
use acacia_proto::packets::{ChangeDimension, MovePlayer, MovePlayerMode, PlayerAction, UpdateSubchunkBlocks};
use common::{Harness, all, hash, ids};
use sapling_api::{Block, BlockPos, GameMode, PlayerId};
use sapling_world::{NETHER, World};

const ALEX: PlayerId = PlayerId(1);

/// A Nether with a netherrack floor at y 40 round the origin: room for a portal there.
fn nether() -> World {
    let mut nether = World::void_in(NETHER);
    let netherrack = nether.block_id("minecraft:netherrack").unwrap();
    nether.fill([-24, 40, -24], [24, 40, 24], netherrack);
    nether
}

/// An obsidian frame round the 2 x 3 inside from (2, 65, 3) along x, on the platform.
fn frame() -> Vec<BlockPos> {
    (1..=4).flat_map(|x| (64..=69).map(move |y| [x, y, 3])).filter(|[x, y, _]| *x == 1 || *x == 4 || *y == 64 || *y == 68).filter(|[_, y, _]| *y <= 68).collect()
}

fn lit() -> Harness {
    let mut h = Harness::new();
    assert!(h.shard.add_dimension("default", nether()));
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for at in frame() {
            assert!(ctx.set_block(at, &Block::new("minecraft:obsidian")));
        }
    });
    h.tick(1);
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 3], &Block::new("minecraft:fire"))));
    h
}

fn portal_cells(packets: &[UpdateSubchunkBlocks]) -> Vec<([i32; 3], u32)> {
    packets.iter().flat_map(|packet| packet.blocks.iter().map(|b| ([b.position.x, b.position.y, b.position.z], b.runtime_id))).collect()
}

#[test]
fn fire_in_a_frame_lights_it_and_a_broken_frame_empties_it() {
    let mut h = lit();
    // The fire goes in in one tick's handlers; the frame is looked at in the next tick's.
    let mut sent = h.tick(1);
    sent.extend(h.tick(1));
    let portal = sapling_world::BlockRegistry::vanilla().network_hash(sapling_world::BlockRegistry::vanilla().find("minecraft:portal", "portal_axis=x").unwrap());
    let cells = portal_cells(&all::<UpdateSubchunkBlocks>(&sent));
    // Along the frame, each column from the top, as BDS listed them.
    let order = [[2, 67, 3], [2, 66, 3], [2, 65, 3], [3, 67, 3], [3, 66, 3], [3, 65, 3]];
    assert_eq!(cells, order.map(|at| (at, portal)).to_vec());
    h.remote.run(|ctx| assert_eq!(ctx.block([3, 66, 3]).map(|block| block.name), Some("minecraft:portal".into())));
    h.tick(1);

    h.remote.run(|ctx| assert!(ctx.set_block([1, 66, 3], &Block::air())));
    let mut sent = h.tick(1);
    sent.extend(h.tick(1));
    let air = hash("minecraft:air");
    let gone = portal_cells(&all::<UpdateSubchunkBlocks>(&sent));
    assert_eq!(gone, [[2, 67, 3], [3, 67, 3], [2, 66, 3], [3, 66, 3], [2, 65, 3], [3, 65, 3]].map(|at| (at, air)).to_vec(), "the whole sheet, top row first");
}

#[test]
fn a_creative_player_in_a_portal_goes_at_once_and_a_portal_is_made_for_it() {
    let mut h = lit();
    h.tick(1);
    h.tick(1);
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.teleport(ALEX, [2.5, 65.0, 3.5]);
    });
    // A void Nether is there at once: the trip may end in the tick it starts.
    let mut sent = Vec::new();
    for _ in 0..20 {
        sent.extend(h.tick(1));
        if sent.iter().any(|p| p.id == PlayerAction::ID) {
            break;
        }
    }
    // No portal is known in the Nether: the scaled place, x and z to zero, at the eyes' height.
    let change = all::<ChangeDimension>(&sent).remove(0);
    assert_eq!((change.dimension, [change.position.x, change.position.z], change.loading_screen_id), (1, [0.0, 0.0], Some(0)));
    let placed = all::<MovePlayer>(&sent).into_iter().find(|moved| moved.mode == MovePlayerMode::Reset).expect("placed in the new portal");
    // Its columns were not sent yet: the client gets the portal with them, not as an update.
    assert!(all::<UpdateSubchunkBlocks>(&sent).is_empty(), "{:?}", ids(&sent));
    let feet = [placed.position.x, placed.position.y - sapling_data::EYE_HEIGHT, placed.position.z];
    assert_eq!(feet[1], 41.0, "on the floor at y 40");
    h.remote.run(move |ctx| {
        let at = feet.map(|c| c.floor() as i32);
        assert_eq!(ctx.within("default:nether", |ctx| ctx.block(at).map(|block| block.name)).flatten(), Some("minecraft:portal".into()));
        assert_eq!(ctx.within("default:nether", |ctx| ctx.block([at[0], at[1] - 1, at[2]]).map(|block| block.name)).flatten(), Some("minecraft:obsidian".into()));
    });
    h.tick(1);
}
