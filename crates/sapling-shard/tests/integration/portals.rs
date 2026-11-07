//! Nether portals: lit by fire in a frame, gone with the frame, and a trip through one that
//! makes a portal at the other end (research/captures/bds-portal-*.txt).

use crate::common;

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
    // The frame cell that broke it goes in the sheet's batch, as BDS's (research/captures/bds-n13sheet.txt).
    assert_eq!(gone, [[1, 66, 3], [2, 67, 3], [3, 67, 3], [2, 66, 3], [3, 66, 3], [2, 65, 3], [3, 65, 3]].map(|at| (at, air)).to_vec(), "the whole sheet, top row first");
}

#[test]
fn a_broken_sheet_goes_out_by_sub_chunk_in_bds_s_order_with_the_frame_cell_first_in_its_own() {
    // research/captures/bds-n13sheet.txt: a 2 x 14 sheet over two sub-chunks across and two up.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for x in 14..=17 {
            for y in -27..=-12 {
                let inside = (15..=16).contains(&x) && (-26..=-13).contains(&y);
                let block = if inside { Block::new("minecraft:portal").with("portal_axis=x") } else { Block::new("minecraft:obsidian") };
                assert!(ctx.set_block([x, y, 3], &block));
            }
        }
    });
    h.tick_all();
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.update_block([14, -20, 3], &Block::air())));
    let raw: Vec<acacia_proto::RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let sent = all::<UpdateSubchunkBlocks>(&raw);
    let corners: Vec<[i32; 3]> = sent.iter().map(|packet| [packet.x, packet.y, packet.z]).collect();
    assert_eq!(corners, [[16, -32, 0], [16, -16, 0], [0, -16, 0], [0, -32, 0]]);
    let first = &sent[3].blocks[0].position;
    assert_eq!([first.x, first.y, first.z], [14, -20, 3]);
    assert!(all::<acacia_proto::packets::UpdateBlock>(&raw).is_empty(), "the frame cell is not sent alone");
}

#[test]
fn a_big_frame_lights_in_bds_s_order() {
    // research/captures/bds-portal-sections.txt: the x 60 frame, z 20..40 and y -59..-39 inside.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for z in 19..=41 {
            for y in -60..=-38 {
                let rim = z == 19 || z == 41 || y == -60 || y == -38;
                assert!(ctx.set_block([60, y, z], &if rim { Block::new("minecraft:obsidian") } else { Block::air() }));
            }
        }
    });
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.set_block([60, -59, 20], &Block::new("minecraft:fire"))));
    let sent: Vec<UpdateSubchunkBlocks> = (0..3).flat_map(|_| all::<UpdateSubchunkBlocks>(h.tick_all().to(1))).filter(|packet| packet.blocks.len() > 1).collect();
    let corners: Vec<[i32; 3]> = sent.iter().map(|packet| [packet.x, packet.y, packet.z]).collect();
    assert_eq!(corners, [[48, -64, 16], [48, -48, 16], [48, -64, 32], [48, -48, 32]]);
    let first: Vec<[i32; 3]> = sent[0].blocks.iter().take(4).map(|b| [b.position.x, b.position.y, b.position.z]).collect();
    assert_eq!(first, [[60, -49, 30], [60, -50, 30], [60, -49, 29], [60, -50, 29]]);
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
    // Made once the client holds the columns round it, the portal follows the placing as an update.
    let at = |id: u32| sent.iter().position(|packet| packet.id == id);
    assert!(at(UpdateSubchunkBlocks::ID) > at(PlayerAction::ID), "{:?}", ids(&sent));
    let obsidian = hash("minecraft:obsidian");
    assert!(all::<UpdateSubchunkBlocks>(&sent).iter().any(|packet| packet.blocks.iter().any(|b| b.runtime_id == obsidian)), "{:?}", ids(&sent));
    let feet = [placed.position.x, placed.position.y - sapling_data::EYE_HEIGHT, placed.position.z];
    assert_eq!(feet[1], 41.0, "on the floor at y 40");
    h.remote.run(move |ctx| {
        let at = feet.map(|c| c.floor() as i32);
        assert_eq!(ctx.within("default:nether", |ctx| ctx.block(at).map(|block| block.name)).flatten(), Some("minecraft:portal".into()));
        assert_eq!(ctx.within("default:nether", |ctx| ctx.block([at[0], at[1] - 1, at[2]]).map(|block| block.name)).flatten(), Some("minecraft:obsidian".into()));
    });
    h.tick(1);
}
