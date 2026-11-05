//! Blocks against the group V drills: copper golem statues set, put and clicked (bds-v2statue.txt,
//! bds-z2open.txt); teleports and chorus fruit a sculk sensor hears (bds-v2tp.txt).

use crate::common;

use acacia_proto::nbt::Value as NbtValue;
use acacia_proto::packets::{BlockEntityData, CompletedUsingItem, LevelEventGeneric, LevelSoundEvent};
use acacia_proto::types::{TransactionTransactionData, TransactionUseItemActionType};
use acacia_proto::RawPacket;
use common::{Harness, all, block_changes, click};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};

const UP: u8 = 1;
const STATUE: &str = "minecraft:copper_golem_statue";

/// Alex in survival at the spawn holding `held`, or nothing.
fn holding(held: Option<&'static str>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Survival);
        assert!(ctx.set_item(PlayerId(1), Slot::Main(0), held.map(|held| Item::new(held, 16)).as_ref()));
    });
    h.tick_all();
    h
}

/// The statues shown in `sent`: each one's pose, and whether it holds the unnamed golem.
fn statues(sent: &[RawPacket]) -> Vec<(i32, bool)> {
    let golem = NbtValue::Compound(vec![("ActorIdentifier".into(), NbtValue::String("minecraft:copper_golem<>".into())), ("SaveData".into(), NbtValue::Compound(Vec::new()))]);
    all::<BlockEntityData>(sent)
        .into_iter()
        .map(|data| data.nbt.decode().unwrap().value)
        .filter(|entity| entity.get("id") == Some(&NbtValue::String("CopperGolemStatue".into())))
        .map(|entity| (if let Some(NbtValue::Int(pose)) = entity.get("Pose") { *pose } else { -1 }, entity.get("Actor") == Some(&golem)))
        .collect()
}

#[test]
fn a_copper_golem_statue_set_or_put_holds_a_golem_shown_at_once_and_again_a_tick_on() {
    // research/captures/bds-v2statue.txt: set (any stage) or put, block and entity at once and a tick later, pose 0.
    let mut h = holding(Some(STATUE));
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 3], &Block::new("minecraft:waxed_oxidized_copper_golem_statue"))));
    let set: Vec<Vec<(i32, bool)>> = (0..3).map(|_| statues(h.tick_all().to(1))).collect();
    assert_eq!(set, [vec![(0, true)], vec![(0, true)], vec![]]);
    h.receive(1, &click([2, 64, 1], UP, Some(STATUE)));
    let put: Vec<Vec<(i32, bool)>> = (0..3).map(|_| statues(h.tick_all().to(1))).collect();
    assert_eq!(put, [vec![(0, true)], vec![(0, true)], vec![]]);
}

fn sensor_phase(h: &mut Harness) -> String {
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(ctx.block([6, 65, 4]).expect("loaded").properties).expect("the test waits"));
    h.tick_all();
    got.recv().expect("ran")
}

/// The origins of the vibration particles (the LevelEventGenerics with an origin) in `sent`.
fn vibrations(sent: &[RawPacket]) -> Vec<[f32; 3]> {
    all::<LevelEventGeneric>(sent)
        .into_iter()
        .filter_map(|event| {
            let origin = &event.nbt.iter().find(|nbt| nbt.name == "origin")?.value;
            let axis = |key: &str| if let Some(NbtValue::Float(c)) = origin.get(key) { Some(*c) } else { None };
            Some([axis("x")?, axis("y")?, axis("z")?])
        })
        .collect()
}

#[test]
fn a_tp_is_heard_where_it_lands_unless_it_lands_where_it_stood() {
    // research/captures/bds-v2tp.txt: the actor sent to 1.2 -26 3.8 heard from its cell's middle, sent to ~ ~ ~
    // unheard, an armour stand sent to 3.5 -26 5.5 heard; the sensor 6 -25 4.
    let mut h = holding(None);
    h.remote.run(|ctx| assert!(ctx.set_block([6, 65, 4], &Block::new("minecraft:sculk_sensor"))));
    h.tick_all();
    h.remote.run(|ctx| ctx.teleport(PlayerId(1), [1.2, 65.0, 3.8]));
    let sent: Vec<RawPacket> = (0..8).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!(vibrations(&sent), [[1.5, 65.5, 3.5]]);
    assert_eq!(sensor_phase(&mut h), "sculk_sensor_phase=1");
    (0..50).for_each(|_| _ = h.tick_all());
    h.remote.run(|ctx| ctx.teleport(PlayerId(1), [1.2, 65.0, 3.8]));
    let sent: Vec<RawPacket> = (0..8).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!((vibrations(&sent), sensor_phase(&mut h)), (vec![], "sculk_sensor_phase=0".to_owned()), "put where it stood");
    let (made, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| made.send(ctx.spawn_entity("minecraft:armor_stand", [2.5, 65.0, 6.5])).expect("the test waits"));
    h.tick_all();
    let stand = got.recv().expect("ran").expect("a stand");
    (0..3).for_each(|_| _ = h.tick_all());
    h.remote.run(move |ctx| assert!(ctx.move_entity(stand, [3.5, 65.0, 5.5])));
    let sent: Vec<RawPacket> = (0..8).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!(vibrations(&sent), [[3.5, 65.5, 5.5]]);
}

#[test]
fn a_full_player_eats_chorus_fruit_heard_from_its_eyes() {
    // research/captures/bds-v2tp.txt: hunger 20, eaten all the same, heard from the eyes' cell it left.
    let mut h = holding(Some("minecraft:chorus_fruit"));
    let feet = common::asking::ask(&mut h, |ctx| ctx.position(PlayerId(1)).expect("in the world").feet);
    let eyes = [feet[0], feet[1] + 1.62, feet[2]].map(|c| c.floor() + 0.5);
    let sensor = [eyes[0] as i32 + 4, feet[1].floor() as i32, eyes[2] as i32];
    h.remote.run(move |ctx| assert!(ctx.set_block(sensor, &Block::new("minecraft:sculk_sensor"))));
    h.tick_all();
    let mut use_air = click([0, 0, 0], 255, None);
    if let TransactionTransactionData::ItemUse(used) = &mut use_air.transaction.transaction_data {
        used.action_type = TransactionUseItemActionType::ClickAir;
    }
    h.receive(1, &use_air);
    let sent: Vec<RawPacket> = (0..50).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!((all::<CompletedUsingItem>(&sent).len(), vibrations(&sent).first().copied()), (1, Some(eyes)));
}

#[test]
fn a_cloned_shulker_box_is_shown_after_its_block_holding_its_stacks_unpaired() {
    // research/captures/bds-v2ents.txt: the clone's block first, then its entity with the stacks and `forceunpair` 1.
    let mut h = holding(None);
    h.remote.run(|ctx| {
        assert!(ctx.set_block([5, 65, -2], &Block::new("minecraft:undyed_shulker_box")));
        assert!(ctx.set_container_item([5, 65, -2], 0, Some(&Item::new("minecraft:stone", 5))));
    });
    h.tick_all();
    h.remote.run(|ctx| {
        let (block, entity) = (ctx.block([5, 65, -2]).expect("loaded"), ctx.block_entity([5, 65, -2]).expect("a box"));
        ctx.batch_blocks(|ctx| {
            assert!(ctx.update_block([7, 65, -2], &block));
            assert!(ctx.set_block_entity([7, 65, -2], &entity));
        });
    });
    let sent = h.tick_all().to(1).to_vec();
    let first = |shown: fn(&RawPacket) -> bool| sent.iter().position(shown).expect("shown");
    let (block_at, entity_at) = (first(|packet| !block_changes(std::slice::from_ref(packet)).is_empty()), first(|packet| !all::<BlockEntityData>(std::slice::from_ref(packet)).is_empty()));
    assert!(block_at < entity_at, "the block before its entity");
    let held = |entity: &NbtValue| matches!(entity.get("Items"), Some(NbtValue::List(items)) if !items.items.is_empty());
    let shown: Vec<NbtValue> = all::<BlockEntityData>(&sent).into_iter().map(|data| data.nbt.decode().unwrap().value).collect();
    assert!(shown.iter().all(|entity| held(entity) && entity.get("forceunpair") == Some(&NbtValue::Byte(1))), "{shown:?}");
}

#[test]
fn a_shulker_box_put_by_hand_is_shown_twice_before_its_sound_and_once_after() {
    // research/captures/bds-v2ents.txt, bds-p2more.txt.
    let mut h = holding(Some("minecraft:undyed_shulker_box"));
    h.receive(1, &click([2, 64, 1], UP, Some("minecraft:undyed_shulker_box")));
    let sent = h.tick_all().to(1).to_vec();
    let order: Vec<&str> = sent
        .iter()
        .filter_map(|packet| {
            let one = std::slice::from_ref(packet);
            if !all::<BlockEntityData>(one).is_empty() { Some("entity") } else if all::<LevelSoundEvent>(one).iter().any(|sound| sound.sound_id == "place") { Some("sound") } else { None }
        })
        .collect();
    assert_eq!(order, ["entity", "entity", "sound", "entity"]);
}

#[test]
fn an_empty_hand_turns_a_statue_through_its_four_poses_showing_it_at_once_unheard() {
    // research/captures/bds-z2open.txt, bds-v2statue.txt: each click shows the block and the entity, pose + 1, no sound.
    let mut h = holding(None);
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 3], &Block::new(STATUE))));
    (0..3).for_each(|_| _ = h.tick_all());
    let mut poses = Vec::new();
    for _ in 0..4 {
        h.receive(1, &click([2, 65, 3], 4, None));
        let sent = h.tick_all().to(1).to_vec();
        let cells: Vec<[i32; 3]> = block_changes(&sent).into_iter().map(|(at, _)| at).collect();
        assert_eq!((cells, all::<LevelSoundEvent>(&sent).len()), (vec![[2, 65, 3]], 0), "the statue alone, unheard");
        poses.extend(statues(&sent).into_iter().map(|(pose, _)| pose));
    }
    assert_eq!(poses, [1, 2, 3, 0]);
}
