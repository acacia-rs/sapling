//! Players as each other's entities: list, appearance, movement, pose, leaving (harness: common/mod.rs).

use crate::common;

use acacia_proto::Packet;
use acacia_proto::packets::{
    AddPlayer, Animate, AnimateActionId, AvailableCommands, LevelChunk, MovePlayer, MovePlayerMode, NetworkChunkPublisherUpdate, PlayerList, RemoveEntity, SetEntityData,
};
use acacia_proto::types::{
    GameMode, InputData, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1,
    PlayerRecordContent, PlayerRecordContentAdd,
};
use common::{Harness, all, ids, input_at};
use sapling_data::EYE_HEIGHT;

const SPAWN: [f32; 3] = [0.5, 65.0, 0.5];

fn added(list: &PlayerList) -> &PlayerRecordContentAdd {
    match list.records.as_slice() {
        [record] => match &record.content {
            PlayerRecordContent::Add(add) => add,
            other => panic!("not an add: {other:?}"),
        },
        records => panic!("{} records in one list", records.len()),
    }
}

fn flags_of(metadata: &[MetadataDictionaryItem]) -> MetadataFlags1 {
    metadata.iter().find_map(|i| if let Value::Flags(flags) = &i.value { Some(*flags) } else { None }).expect("entity flags")
}

/// Alex (connection and entity 1) and Steve (2), both in the world at the spawn.
fn two_players() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h
}

#[test]
fn players_are_listed_and_shown_to_each_other() {
    let mut h = Harness::new();
    let sent = h.enter(1, "Alex");
    let own: PlayerList = sent.to(1)[0].decode().unwrap();
    assert_eq!((ids(sent.to(1)), added(&own).username.as_str()), (vec![PlayerList::ID, AvailableCommands::ID], "Alex"));

    let sent = h.enter(2, "Steve");
    assert_eq!(ids(sent.to(1)), [PlayerList::ID, AddPlayer::ID]);
    assert_eq!(ids(sent.to(2)), [PlayerList::ID, PlayerList::ID, AddPlayer::ID, AvailableCommands::ID]);
    let (steve_listed, alex_listed) = (all::<PlayerList>(sent.to(1)), all::<PlayerList>(sent.to(2)));
    let (steve, alex) = (added(&steve_listed[0]), added(&alex_listed[0]));
    assert_eq!((alex.username.as_str(), steve.username.as_str(), added(&alex_listed[1])), ("Alex", "Steve", steve));
    assert_eq!((alex.entity_unique_id, steve.entity_unique_id, steve.xbox_user_id.as_str(), steve.build_platform), (1, 2, "", 1));
    let skin = &steve.skin_data;
    assert!(skin.skin_id.starts_with("persona-") && skin.skin_data.data.len() == 256 * 256 * 4, "the login's skin is forwarded: {}", skin.skin_id);
    assert_eq!((steve_listed[0].records[0].legacy_type, skin.trusted.as_str()), (0, "true"));

    let shown: AddPlayer = sent.to(2)[2].decode().unwrap();
    assert_eq!((shown.username.as_str(), shown.runtime_id, shown.unique_id, shown.uuid), ("Alex", 1, 1, alex.uuid));
    assert_eq!(([shown.position.x, shown.position.y, shown.position.z], shown.gamemode), (SPAWN, GameMode::Fallback), "AddPlayer places the feet");
    let shown: AddPlayer = sent.to(1)[1].decode().unwrap();
    assert_eq!((shown.username.as_str(), shown.runtime_id, shown.uuid), ("Steve", 2, steve.uuid));
    assert_eq!(h.log(), ["join Alex (1 online)", "join Steve (2 online)"]);
    assert!(h.tick_all().to(1).is_empty() && h.tick_all().to(2).is_empty(), "nothing more until something changes");
}

#[test]
fn a_move_reaches_the_viewer_once_and_not_the_mover() {
    let mut h = two_players();
    let mut input = input_at([2.5, 65.0, 0.5]);
    (input.yaw, input.head_yaw, input.pitch, input.tick) = (90.0, 95.0, 10.0, 77);
    input.input_data = vec![InputData::VerticalCollision];
    h.receive(1, &input);
    let sent = h.tick_all();
    assert!(sent.to(1).is_empty(), "the mover is not told about itself");
    assert_eq!(ids(sent.to(2)), [MovePlayer::ID]);
    let moved: MovePlayer = sent.to(2)[0].decode().unwrap();
    assert_eq!((moved.runtime_id, moved.mode, moved.on_ground, moved.teleport.is_none(), moved.tick), (1, MovePlayerMode::Normal, true, true, 77));
    assert_eq!(([moved.position.x, moved.position.y, moved.position.z], moved.yaw, moved.head_yaw, moved.pitch), ([2.5, 65.0 + EYE_HEIGHT, 0.5], 90.0, 95.0, 10.0));

    // The same input again changes nothing; two moves in one tick are told as one.
    h.receive(1, &input);
    assert!(h.tick_all().to(2).is_empty());
    h.receive(1, &input_at([3.0, 65.0, 0.5]));
    h.receive(1, &input_at([3.5, 65.0, 0.5]));
    let moves = all::<MovePlayer>(h.tick_all().to(2));
    assert_eq!((moves.len(), moves[0].position.x, moves[0].on_ground), (1, 3.5, false));
}

#[test]
fn pose_and_swings_reach_the_viewer() {
    let mut h = two_players();
    let mut input = input_at(SPAWN);
    input.input_data = vec![InputData::StartSneaking, InputData::StartSprinting];
    h.receive(1, &input);
    let sent = h.tick_all();
    assert!(sent.to(1).is_empty());
    let data = all::<SetEntityData>(sent.to(2)).remove(0);
    let flags = flags_of(&data.metadata);
    assert!(flags.contains(MetadataFlags1::SNEAKING) && flags.contains(MetadataFlags1::SPRINTING) && !flags.contains(MetadataFlags1::SWIMMING));
    assert!(flags.contains(MetadataFlags1::HAS_COLLISION), "the recorded flags stay set");
    let height = data.metadata.iter().find(|i| i.key == Key::BoundingboxHeight).map(|i| i.value.clone());
    assert_eq!((data.runtime_entity_id, data.metadata.len(), height), (1, 2, Some(Value::Default(Plain::Float(1.49)))));

    input.input_data = vec![InputData::StopSneaking];
    h.receive(1, &input);
    let sent = h.tick_all();
    assert_eq!(ids(sent.to(2)), [SetEntityData::ID], "standing still, only the pose is told");
    let flags = flags_of(&all::<SetEntityData>(sent.to(2))[0].metadata);
    assert!(!flags.contains(MetadataFlags1::SNEAKING) && flags.contains(MetadataFlags1::SPRINTING));

    // A swing is relayed under the sender's own id, once a tick.
    let swing = Animate { action_id: AnimateActionId::SwingArm, runtime_entity_id: 2, data: 3.0, has_swing_source: true, swing_source: Some("x".into()) };
    h.receive(1, &swing);
    h.receive(1, &swing);
    h.receive(1, &Animate { action_id: AnimateActionId::CriticalHit, ..swing.clone() });
    let sent = h.tick_all();
    let relayed = Animate { action_id: AnimateActionId::SwingArm, runtime_entity_id: 1, data: 0.0, has_swing_source: false, swing_source: None };
    assert_eq!((all::<Animate>(sent.to(2)), sent.to(1).len()), (vec![relayed], 0));

    // Someone who arrives later sees the pose in AddPlayer.
    let sent = h.enter(3, "Late");
    let alex = all::<AddPlayer>(sent.to(3)).into_iter().find(|p| p.runtime_id == 1).expect("Alex");
    let flags = flags_of(&alex.metadata);
    assert!(flags.contains(MetadataFlags1::SPRINTING) && !flags.contains(MetadataFlags1::SNEAKING));
}

#[test]
fn a_teleport_sized_step_is_sent_back() {
    let mut h = two_players();
    h.receive(1, &input_at([500.5, 65.0, 0.5]));
    let sent = h.tick_all();
    assert!(sent.to(2).is_empty(), "the viewer never sees the jump");
    assert_eq!(ids(sent.to(1)), [MovePlayer::ID], "no chunks around the claimed position either");
    let back: MovePlayer = sent.to(1)[0].decode().unwrap();
    assert_eq!((back.runtime_id, back.mode, [back.position.x, back.position.y, back.position.z]), (1, MovePlayerMode::Teleport, [0.5, 65.0 + EYE_HEIGHT, 0.5]));
    assert!(back.teleport.is_some());

    // While the client catches up the correction is not repeated; a sane step is taken again.
    h.receive(1, &input_at([500.5, 65.0, 0.5]));
    assert!(h.tick_all().to(1).is_empty());
    h.receive(1, &input_at([f32::NAN, 65.0, 0.5]));
    h.receive(1, &input_at([1.5, 65.0, 0.5]));
    let sent = h.tick_all();
    assert_eq!((all::<MovePlayer>(sent.to(2)).len(), sent.to(1).len()), (1, 0));
    (0..20).for_each(|_| drop(h.tick_all()));
    h.receive(1, &input_at([-400.0, 65.0, 0.5]));
    assert_eq!(ids(h.tick_all().to(1)), [MovePlayer::ID]);
}

#[test]
fn leaving_the_view_removes_the_entity_and_coming_back_adds_it() {
    let mut h = two_players();
    // Six chunks east in steps a player can make; views are four chunks wide.
    (1..=11).for_each(|step| h.receive(1, &input_at([0.5 + 9.0 * step as f32, 65.0, 0.5])));
    let sent = h.tick_all();
    assert_eq!(all::<RemoveEntity>(sent.to(1)), [RemoveEntity { entity_id_self: 2 }]);
    assert_eq!(ids(sent.to(2)), [RemoveEntity::ID], "removed, not moved");
    assert_eq!(all::<RemoveEntity>(sent.to(2)), [RemoveEntity { entity_id_self: 1 }]);
    let terrain = [NetworkChunkPublisherUpdate::ID, LevelChunk::ID, RemoveEntity::ID];
    assert!(ids(sent.to(1)).iter().all(|id| terrain.contains(id)));

    h.receive(1, &input_at([99.5, 65.0, 5.5]));
    assert!(h.tick_all().to(2).is_empty(), "out of view, nothing is told");

    (0..=11).rev().for_each(|step| h.receive(1, &input_at([0.5 + 9.0 * step as f32, 65.0, 0.5])));
    let sent = h.tick_all();
    let back = all::<AddPlayer>(sent.to(2));
    assert_eq!((back.len(), back[0].runtime_id, back[0].position.x), (1, 1, 0.5));
    assert_eq!(all::<AddPlayer>(sent.to(1))[0].runtime_id, 2);
    assert!(all::<PlayerList>(sent.to(1)).is_empty() && all::<PlayerList>(sent.to(2)).is_empty(), "the list does not depend on the view");
}

#[test]
fn quitting_removes_the_entity_and_the_list_entry() {
    let mut h = two_players();
    let listed: PlayerList = h.enter(3, "Late").to(2)[0].decode().unwrap();
    let late = added(&listed).uuid;
    h.leave(3);
    let sent = h.tick_all();
    for conn in [1, 2] {
        assert_eq!(ids(sent.to(conn)), [RemoveEntity::ID, PlayerList::ID]);
        assert_eq!(all::<RemoveEntity>(sent.to(conn)), [RemoveEntity { entity_id_self: 3 }]);
        let list: PlayerList = sent.to(conn)[1].decode().unwrap();
        let PlayerRecordContent::Remove(removed) = &list.records[0].content else { panic!("not a remove: {list:?}") };
        assert_eq!((list.records.len(), list.records[0].legacy_type, removed.uuid), (1, 1, late));
    }
    assert!(sent.to(3).is_empty());
    assert_eq!(h.log().last().map(String::as_str), Some("quit Late (2 online)"));

    // A player who never finished joining was never listed.
    h.join(4, "Early");
    h.tick_all();
    h.leave(4);
    let sent = h.tick_all();
    assert!(sent.to(1).is_empty() && sent.to(2).is_empty());
}
