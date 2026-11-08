//! Blocks against the group T drills: string's shape as it is put or set (bds-z2wire.txt), a shulker box
//! and entity blocks put by hand (bds-p2place.txt, bds-z2hand.txt), wool and sensors (bds-z2sculk.txt).

use crate::common;

use std::sync::mpsc;

use acacia_proto::nbt::Value as NbtValue;
use acacia_proto::packets::{AddItemEntity, BlockEntityData, LevelEventGeneric, LevelSoundEvent, UpdateSubchunkBlocks};
use acacia_proto::{Packet, RawPacket};
use common::{Harness, all, block_changes, click};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};

const UP: u8 = 1;
const WIRE: &str = "minecraft:trip_wire";

fn block(h: &mut Harness, pos: [i32; 3]) -> Block {
    let (sent, got) = mpsc::channel();
    h.remote.run(move |ctx| sent.send(ctx.block(pos).expect("loaded")).expect("the test waits"));
    h.tick_all();
    got.recv().expect("ran")
}

fn hash(block: &Block) -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.find(&block.name, &block.properties).expect("a state"))
}

/// Alex in survival at the spawn holding `held`.
fn holding(held: &'static str) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Survival);
        assert!(ctx.set_item(PlayerId(1), Slot::Main(0), Some(&Item::new(held, 16))));
    });
    h.tick_all();
    h
}

fn has(block: &Block, bits: &[&str]) -> bool {
    bits.iter().all(|bit| block.properties.split(',').any(|pair| pair == *bit))
}

#[test]
fn string_put_between_two_hooks_is_joined_both_ways_suspended_attached_and_shown_so() {
    let mut h = holding("minecraft:string");
    h.remote.run(|ctx| {
        for (pos, block) in [([0, 65, 3], Block::new("minecraft:stone")), ([4, 65, 3], Block::new("minecraft:stone")), ([1, 65, 3], Block::new("minecraft:tripwire_hook").with("direction=3")), ([3, 65, 3], Block::new("minecraft:tripwire_hook").with("direction=1"))] {
            assert!(ctx.set_block(pos, &block));
        }
    });
    h.tick_all();
    h.receive(1, &click([2, 64, 3], UP, Some(WIRE)));
    let sent: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let wire = block(&mut h, [2, 65, 3]);
    assert!(wire.name == WIRE && has(&wire, &["minecraft:connection_east=1", "minecraft:connection_west=1", "minecraft:connection_north=0", "suspended_bit=1", "attached_bit=1"]), "{wire:?}");
    let shown = block_changes(&sent).into_iter().filter(|(at, _)| *at == [2, 65, 3]).map(|(_, id)| id).last();
    assert_eq!(shown, Some(hash(&wire)), "the client is left with the wire as it is");
    let attached: Vec<[i32; 3]> = all::<LevelSoundEvent>(&sent).into_iter().filter(|sound| sound.sound_id == "attach").map(|sound| [sound.position.x, sound.position.y, sound.position.z].map(|c| (c * 10.0).round() as i32)).collect();
    assert_eq!(attached, [[35, 651, 35], [15, 651, 35]], "each hook heard 0.1 over its cell, the far one first");
}

#[test]
fn a_shulker_box_put_by_hand_is_listed_twice_in_one_update() {
    // research/captures/bds-p2place.txt: UpdateSubchunkBlocks 3 -25 -2 undyed_shulker_box twice.
    let mut h = holding("minecraft:undyed_shulker_box");
    h.receive(1, &click([2, 64, 3], UP, Some("minecraft:undyed_shulker_box")));
    let sent = h.tick_all().to(1).to_vec();
    let listed: Vec<Vec<[i32; 3]>> = all::<UpdateSubchunkBlocks>(&sent).into_iter().map(|update| update.blocks.iter().map(|cell| [cell.position.x, cell.position.y, cell.position.z]).collect()).collect();
    assert_eq!(listed, [vec![[2, 65, 3], [2, 65, 3]]]);
}

/// The ids of the entities shown in `sent`.
fn entities_shown(sent: &[RawPacket]) -> Vec<String> {
    all::<BlockEntityData>(sent).into_iter().filter_map(|data| if let Some(NbtValue::String(id)) = data.nbt.decode().unwrap().value.get("id") { Some(id.to_string()) } else { None }).collect()
}

#[test]
fn a_furnace_put_by_hand_shows_no_entity_and_a_jukebox_shows_its_own_then_again_finished() {
    // research/captures/bds-z2hand.txt: no BlockEntityData for a furnace, dispenser, hopper or brewing stand
    // put; a jukebox's before its place sound, and again a tick on with FinishedRecording 1.
    let mut h = holding("minecraft:furnace");
    h.receive(1, &click([2, 64, 3], UP, Some("minecraft:furnace")));
    assert_eq!(entities_shown(h.tick_all().to(1)), Vec::<String>::new());
    h.remote.run(|ctx| assert!(ctx.set_item(PlayerId(1), Slot::Main(0), Some(&Item::new("minecraft:jukebox", 1)))));
    h.tick_all();
    h.receive(1, &click([2, 64, -3], UP, Some("minecraft:jukebox")));
    let put = h.tick_all().to(1).to_vec();
    assert_eq!(entities_shown(&put), ["Jukebox"]);
    let order: Vec<u32> =put.iter().map(|packet| packet.id).filter(|id| [BlockEntityData::ID, LevelSoundEvent::ID].contains(id)).collect();
    assert_eq!(order, [BlockEntityData::ID, LevelSoundEvent::ID], "the entity before the sound");
    let again = h.tick_all().to(1).to_vec();
    let finished = all::<BlockEntityData>(&again).into_iter().map(|data| data.nbt.decode().unwrap().value.get("FinishedRecording").cloned()).collect::<Vec<_>>();
    assert_eq!(finished, [Some(NbtValue::Byte(1))]);
}

#[test]
fn a_hanging_sign_put_under_a_block_hangs_and_is_shown_again_a_tick_on() {
    // research/captures/bds-z2hand.txt: hanging=1 under a block, its entity shown when put and a tick later.
    let mut h = holding("minecraft:oak_hanging_sign");
    h.remote.run(|ctx| assert!(ctx.set_block([2, 67, 3], &Block::new("minecraft:stone"))));
    h.tick_all();
    h.receive(1, &click([2, 67, 3], 0, Some("minecraft:oak_hanging_sign")));
    let shown: Vec<usize> = (0..3).map(|_| entities_shown(h.tick_all().to(1)).len()).collect();
    assert_eq!(shown, [1, 1, 0]);
    assert!(has(&block(&mut h, [2, 66, 3]), &["hanging=1", "ground_sign_direction=0"]));
}

#[test]
fn a_disc_put_in_shows_the_jukebox_a_tick_on_and_one_taken_out_shows_it_finished_at_once() {
    // research/captures/bds-z2open.txt: no entity at once for a disc going in; one coming out showed FinishedRecording 1.
    let mut h = holding("minecraft:music_disc_13");
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 3], &Block::new("minecraft:jukebox"))));
    (0..2).for_each(|_| _ = h.tick_all());
    h.receive(1, &click([2, 65, 3], UP, None));
    let put: Vec<usize> = (0..2).map(|_| entities_shown(h.tick_all().to(1)).len()).collect();
    assert_eq!(put, [0, 1]);
    h.receive(1, &click([2, 65, 3], UP, None));
    let out = h.tick_all().to(1).to_vec();
    let finished: Vec<_> = all::<BlockEntityData>(&out).into_iter().map(|data| data.nbt.decode().unwrap().value.get("FinishedRecording").cloned()).collect();
    assert_eq!(finished, [Some(NbtValue::Byte(1))]);
}

#[test]
fn a_crafter_crafting_shows_five_ticks_left_and_sounds_from_its_corner_as_it_was_fired() {
    // research/captures/bds-z2open.txt: crafting_ticks_remaining 5 with crafting=1; crafter.craft at 5.0 -26.0 4.0
    // naming crafter[crafting=0,orientation=east_up,triggered_bit=1].
    let mut h = holding("minecraft:stone");
    h.remote.run(|ctx| {
        assert!(ctx.set_block([5, 65, 4], &Block::new("minecraft:crafter").with("orientation=east_up")));
        assert!(ctx.set_container_item([5, 65, 4], 0, Some(&Item::new("minecraft:oak_log", 1))));
    });
    (0..2).for_each(|_| _ = h.tick_all());
    h.remote.run(|ctx| assert!(ctx.set_block([5, 65, 5], &Block::new("minecraft:redstone_block"))));
    let sent: Vec<RawPacket> = (0..8).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let crafted: Vec<([f32; 3], i32)> = all::<LevelSoundEvent>(&sent).into_iter().filter(|sound| sound.sound_id == "crafter.craft").map(|sound| ([sound.position.x, sound.position.y, sound.position.z], sound.extra_data)).collect();
    assert_eq!(crafted, [([5.0, 65.0, 4.0], 1228006179)]);
    let remaining: Vec<_> = all::<BlockEntityData>(&sent).into_iter().filter_map(|data| data.nbt.decode().unwrap().value.get("crafting_ticks_remaining").cloned()).collect();
    assert!(remaining.contains(&NbtValue::Int(5)), "{remaining:?}");
}

#[test]
fn a_broken_block_throws_each_item_of_its_loot_alone() {
    // research/captures/bds-z2hand.txt: two AddItemEntity of one charcoal each, for each `setblock ... destroy`;
    // bds-loot.txt: 2 to 4 glowstone dust as as many entities; bds-an1farm.txt: a ripe crop's seeds and wheat.
    let mut h = holding("minecraft:stone");
    for (block, least) in [(Block::new("minecraft:campfire"), 2), (Block::new("minecraft:glowstone"), 2), (Block::new("minecraft:wheat").with("growth=7"), 2)] {
        h.remote.run(move |ctx| assert!(ctx.set_block([3, 65, 3], &block) && ctx.break_block([3, 65, 3])));
        let counts: Vec<u8> = all::<AddItemEntity>(h.tick_all().to(1)).into_iter().map(|item| item.item.count as u8).collect();
        assert!(counts.len() >= least && counts.iter().all(|&count| count == 1), "{counts:?}");
    }
}

fn sensor_phase(h: &mut Harness) -> String {
    block(h, [6, 65, -4]).properties
}

fn put(h: &mut Harness, item: &'static str, on: [i32; 3]) {
    h.remote.run(move |ctx| assert!(ctx.set_item(PlayerId(1), Slot::Main(0), Some(&Item::new(item, 1)))));
    h.tick_all();
    h.receive(1, &click(on, UP, Some(item)));
    (0..12).for_each(|_| _ = h.tick_all());
}

#[test]
fn a_wool_wall_stops_a_vibration_and_carpet_put_is_not_heard() {
    // research/captures/bds-z2sculk.txt: stone put behind wool unheard, then heard with the wall gone; carpet unheard.
    let mut h = holding("minecraft:stone");
    h.remote.run(|ctx| {
        assert!(ctx.set_block([6, 65, -4], &Block::new("minecraft:sculk_sensor")));
        for (y, z) in (65..=67).flat_map(|y| (-6..=-2).map(move |z| (y, z))) {
            assert!(ctx.set_block([4, y, z], &Block::new("minecraft:white_wool")));
        }
    });
    put(&mut h, "minecraft:stone", [2, 64, -4]);
    assert_eq!(sensor_phase(&mut h), "sculk_sensor_phase=0", "behind wool");
    h.remote.run(|ctx| (65..=67).flat_map(|y| (-6..=-2).map(move |z| (y, z))).for_each(|(y, z)| assert!(ctx.set_block([4, y, z], &Block::new("minecraft:air")))));
    put(&mut h, "minecraft:white_carpet", [2, 64, -3]);
    assert_eq!(sensor_phase(&mut h), "sculk_sensor_phase=0", "carpet");
    put(&mut h, "minecraft:stone", [2, 64, -2]);
    assert_eq!(sensor_phase(&mut h), "sculk_sensor_phase=1", "heard");
}

#[test]
fn a_block_put_near_a_sensor_is_heard_after_its_place_sound() {
    // research/captures/bds-z2sculk.txt: the place sound, then the vibration particle (LevelEventGeneric).
    let mut h = holding("minecraft:stone");
    h.remote.run(|ctx| assert!(ctx.set_block([6, 65, -4], &Block::new("minecraft:sculk_sensor"))));
    h.tick_all();
    h.receive(1, &click([2, 64, -2], UP, Some("minecraft:stone")));
    let order: Vec<u32> = h.tick_all().to(1).iter().map(|packet| packet.id).filter(|id| [LevelSoundEvent::ID, LevelEventGeneric::ID].contains(id)).collect();
    assert_eq!(order, [LevelSoundEvent::ID, LevelEventGeneric::ID]);
}

#[test]
fn string_set_alone_is_suspended_and_set_beside_string_joins_it() {
    let mut h = holding("minecraft:string");
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, -3], &Block::new(WIRE))));
    h.tick_all();
    assert!(has(&block(&mut h, [2, 65, -3]), &["minecraft:connection_east=0", "minecraft:connection_west=0", "suspended_bit=1"]));
    h.remote.run(|ctx| assert!(ctx.set_block([3, 65, -3], &Block::new(WIRE))));
    h.tick_all();
    assert!(has(&block(&mut h, [2, 65, -3]), &["minecraft:connection_east=1", "minecraft:connection_west=0"]));
    assert!(has(&block(&mut h, [3, 65, -3]), &["minecraft:connection_west=1", "minecraft:connection_east=0", "suspended_bit=1"]));
}
