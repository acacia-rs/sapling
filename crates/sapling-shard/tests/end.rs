//! A world's End: travel there by command as BDS makes it (research/captures/bds-end-travel.txt).

mod common;

use acacia_proto::Packet;
use acacia_proto::packets::{ChangeDimension, CorrectPlayerMovePrediction, LevelChunk, LevelSoundEvent, MovePlayer, MovePlayerMode, PlayerAction, Respawn, UpdateSubchunkBlocks};
use acacia_proto::types::Vec3f;
use common::{Harness, all, click, hash, ids, input_at};
use sapling_api::{Block, Item, PlayerId, Slot};
use sapling_world::{END, World};

const ALEX: PlayerId = PlayerId(1);

/// A void End with an end stone floor at y 48 round (100, 0).
pub fn end() -> World {
    let mut end = World::void_in(END);
    let end_stone = end.block_id("minecraft:end_stone").unwrap();
    end.fill([84, 40, -16], [116, 48, 16], end_stone);
    end
}

#[test]
fn a_move_into_the_end_by_command_goes_as_into_the_nether() {
    let mut h = Harness::new();
    assert!(h.shard.add_dimension("default", end()));
    assert!(!h.shard.add_dimension("default", end()), "a world has one End");
    h.enter(1, "Alex");
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, END, [100.5, 49.0, 0.5])));
    let sent = h.tick_all();
    let change = all::<ChangeDimension>(sent.to(1)).remove(0);
    assert_eq!((change.dimension, [change.position.x, change.position.y, change.position.z], change.respawn, change.loading_screen_id), (2, [100.5, 49.0, 0.5], false, Some(0)));
    h.receive(1, &input_at([100.5, 49.0, 0.5]));
    let mut seen = Vec::new();
    for _ in 0..10 {
        seen.extend(h.tick_all().to(1).iter().cloned());
        if seen.iter().any(|p| p.id == PlayerAction::ID) {
            break;
        }
    }
    let columns = all::<LevelChunk>(&seen);
    assert!(!columns.is_empty() && columns.iter().all(|column| column.dimension == END), "{:?}", ids(&seen));
    assert!(columns.iter().any(|column| (column.x, column.z) == (6, 0) && column.highest_subchunk_count == Some(4)), "sections up to y 63 are asked for, as BDS's (6, 0)");
    h.remote.run(|ctx| {
        assert_eq!((ctx.world_of(ALEX).as_deref(), ctx.dimension_of(ALEX)), (Some("default:the_end"), Some(2)));
        assert!(ctx.travel(ALEX, 0, [0.5, 65.0, 0.5]));
    });
    let sent = h.tick_all();
    assert_eq!(all::<ChangeDimension>(sent.to(1)).remove(0).loading_screen_id, Some(1));
}

#[test]
fn the_dead_of_the_end_rise_at_the_overworlds_spawn_as_on_bds() {
    let mut h = Harness::new();
    assert!(h.shard.add_dimension("default", end()));
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, END, [100.5, 49.0, 0.5])));
    (0..5).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, sapling_api::GameMode::Survival);
        assert!(ctx.kill_player(ALEX));
    });
    h.tick_all();
    h.receive(1, &Respawn { position: Vec3f { x: 0.0, y: 0.0, z: 0.0 }, state: 2, runtime_entity_id: 0 });
    let sent = h.tick_all();
    let change = all::<ChangeDimension>(sent.to(1)).remove(0);
    assert_eq!((change.dimension, [change.position.x, change.position.y, change.position.z], change.loading_screen_id), (0, [0.0, 32767.0, 0.0], None), "research/captures/bds-end-death-none.txt");
    assert_eq!(all::<PlayerAction>(sent.to(1)).len(), 1, "told at once that it is there");
    let risen: Vec<(u8, [f32; 3])> = all::<Respawn>(sent.to(1)).into_iter().map(|respawn| (respawn.state, [respawn.position.x, respawn.position.y, respawn.position.z])).collect();
    assert_eq!(risen, [(0, [0.0, 32767.0, 0.0]), (1, [0.5, 65.0 + sapling_data::EYE_HEIGHT, 0.5])]);
    h.remote.run(|ctx| assert_eq!((ctx.world_of(ALEX).as_deref(), ctx.dimension_of(ALEX)), (Some("default"), Some(0))));
    h.tick_all();
}

#[test]
fn a_generated_gateway_sends_a_player_over_its_exit_at_once() {
    let mut h = Harness::new();
    assert!(h.shard.add_dimension("default", end()));
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, END, [96.5, 49.0, 4.5])));
    (0..5).for_each(|_| drop(h.tick_all()));
    h.receive(1, &dimension_ack());
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.within("default:the_end", |ctx| ctx.set_block([96, 49, 8], &Block::new("minecraft:end_gateway"))).unwrap_or(false)));
    h.tick_all();
    h.remote.run(|ctx| ctx.teleport(ALEX, [96.5, 49.0, 8.5]));
    let mut sent = Vec::new();
    (0..3).for_each(|_| sent.extend(h.tick(1)));
    let moved: Vec<[f32; 3]> = all::<MovePlayer>(&sent).into_iter().filter(|moved| moved.mode == MovePlayerMode::Teleport).map(|moved| [moved.position.x, moved.position.y - sapling_data::EYE_HEIGHT, moved.position.z]).collect();
    assert_eq!(moved, [[96.5, 49.0, 8.5], [100.5, 51.0, 0.5], [100.5, 51.0, 0.5]], "the generated exit, told twice");
    let shown = all::<acacia_proto::packets::UpdateBlock>(&sent);
    assert_eq!(shown.iter().map(|update| ([update.position.x, update.position.y, update.position.z], update.flags.0)).collect::<Vec<_>>(), [([96, 49, 8], 19)]);
}

fn dimension_ack() -> PlayerAction {
    let origin = acacia_proto::types::BlockCoordinates { x: 0, y: 0, z: 0 };
    PlayerAction { runtime_entity_id: 1, action: acacia_proto::types::Action::DimensionChangeAck, position: origin.clone(), result_position: origin, face: 0 }
}

/// Twelve frames facing the 3 x 3 round (3, 65, 3), every eye in but (3, 65, 1)'s.
fn ring(h: &mut Harness) {
    h.remote.run(|ctx| {
        let sides: [([i32; 2], &str); 4] = [([0, -2], "south"), ([0, 2], "north"), ([-2, 0], "east"), ([2, 0], "west")];
        for ([dx, dz], facing) in sides {
            for i in -1..=1 {
                let at = [3 + dx + i * dz.abs() / 2, 65, 3 + dz + i * dx.abs() / 2];
                let eye = u8::from(at != [3, 65, 1]);
                let frame = Block { name: "minecraft:end_portal_frame".into(), properties: format!("end_portal_eye_bit={eye},minecraft:cardinal_direction={facing}") };
                assert!(ctx.set_block(at, &frame), "{at:?}");
            }
        }
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:ender_eye", 2))));
    });
    h.tick_all();
}

#[test]
fn the_last_eye_makes_the_portal_and_the_portal_takes_a_player_to_the_platform() {
    let mut h = Harness::new();
    assert!(h.shard.add_dimension("default", end()));
    h.enter(1, "Alex");
    ring(&mut h);
    h.receive(1, &click([3, 65, 1], 1, None));
    let sent = h.tick_all();
    let shown: Vec<([i32; 3], u32)> = all::<UpdateSubchunkBlocks>(sent.to(1)).iter().flat_map(|update| update.blocks.iter().map(|b| ([b.position.x, b.position.y, b.position.z], b.runtime_id))).collect();
    let portal = hash("minecraft:end_portal");
    let registry = sapling_world::BlockRegistry::vanilla();
    let filled = registry.network_hash(registry.find("minecraft:end_portal_frame", "end_portal_eye_bit=1,minecraft:cardinal_direction=south").unwrap());
    let mut inside = vec![([3, 65, 1], filled)];
    inside.extend([4, 3, 2].iter().flat_map(|x| [4, 3, 2].map(|z| ([*x, 65, z], portal))));
    assert_eq!(shown, inside, "the frame, then the inside from the far corner, as BDS");
    let sounds: Vec<String> = all::<LevelSoundEvent>(sent.to(1)).into_iter().map(|sound| sound.sound_id).collect();
    assert_eq!(sounds, ["block.end_portal.spawn", "block.end_portal_frame.fill"]);

    h.remote.run(|ctx| ctx.teleport(ALEX, [3.5, 65.0, 3.5]));
    let mut sent = Vec::new();
    for _ in 0..20 {
        sent.extend(h.tick(1));
        if sent.iter().any(|p| p.id == PlayerAction::ID) {
            break;
        }
    }
    let change = all::<ChangeDimension>(&sent).remove(0);
    assert_eq!((change.dimension, [change.position.x, change.position.y, change.position.z]), (2, [100.0, 50.0, 0.0]));
    let correction = all::<CorrectPlayerMovePrediction>(&sent).remove(0);
    assert_eq!(([correction.position.x, correction.position.y - sapling_data::EYE_HEIGHT, correction.position.z], correction.delta.y), ([100.0, 50.0, 0.0], 0.0));
    let placed = all::<MovePlayer>(&sent).into_iter().find(|moved| moved.mode == MovePlayerMode::Reset).expect("put on the platform");
    assert_eq!([placed.position.x, placed.position.y - sapling_data::EYE_HEIGHT, placed.position.z], [100.0, 50.0, 0.0]);
    h.tick(1);
    h.remote.run(|ctx| {
        let mut block = |at| ctx.within("default:the_end", |ctx| ctx.block(at).map(|block| block.name)).flatten();
        assert_eq!((block([98, 49, -2]), block([102, 49, 2]), block([100, 48, 0])), (Some("minecraft:obsidian".into()), Some("minecraft:obsidian".into()), Some("minecraft:end_stone".into())));
        assert_eq!((block([100, 50, 0]), block([100, 52, 0])), (Some("minecraft:air".into()), Some("minecraft:air".into())));
    });
    h.tick(1);
}
