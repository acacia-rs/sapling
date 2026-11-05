//! Group Z's drills (research/captures/bds-x2clone.txt, bds-x2ran.txt, bds-x2drink.txt): a fence set beside
//! blocks listed twice joined, a set vault woken the next tick, a cloned vault connected to nobody, a sensor's
//! power sound after its block.

use crate::common;

use std::sync::mpsc;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value;
use acacia_proto::packets::{BlockEntityData, LevelSoundEvent, UpdateSubchunkBlocks};
use common::{Harness, all, block_changes, click};
use sapling_api::{Block, Copied};

const VAULT: [i32; 3] = [2, 65, 0];

fn ready() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.tick_all();
    h
}

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

fn put(h: &mut Harness, pos: [i32; 3], name: &'static str) {
    h.remote.run(move |ctx| assert!(ctx.update_block(pos, &Block::new(name))));
}

#[test]
fn a_fence_set_between_blocks_is_listed_twice_joined_in_one_update() {
    // research/captures/bds-x2clone.txt: `/setblock 2 -27 5 oak_fence` in the floor (over water), one
    // UpdateSubchunkBlocks listing it twice, joined on all four sides both times.
    let mut h = ready();
    let fence = [2, 64, 5];
    put(&mut h, fence, "minecraft:water");
    h.tick_all();
    put(&mut h, fence, "minecraft:oak_fence");
    let sent = h.tick_all().to(1).to_vec();
    let joined = hash(&block(&mut h, fence));
    assert!(block(&mut h, fence).properties.split(',').all(|pair| pair.ends_with("=1")), "joined all round");
    let shown: Vec<u32> = block_changes(&sent).into_iter().filter(|(at, _)| *at == fence).map(|(_, id)| id).collect();
    assert_eq!(shown, [joined, joined]);
    assert_eq!(all::<UpdateSubchunkBlocks>(&sent).len(), 1, "both in one packet");
}

#[test]
fn a_fence_set_alone_goes_out_once() {
    // research/captures/bds-p2place.txt: `/setblock 4 -26 3 oak_fence` on the floor, nothing beside it: one UpdateBlock.
    let mut h = ready();
    let fence = [4, 65, 3];
    put(&mut h, fence, "minecraft:oak_fence");
    let sent = h.tick_all().to(1).to_vec();
    assert_eq!(block_changes(&sent).into_iter().filter(|(at, _)| *at == fence).count(), 1);
    assert!(all::<UpdateSubchunkBlocks>(&sent).is_empty());
}

fn vault_state(h: &mut Harness, pos: [i32; 3]) -> String {
    block(h, pos).properties.split(',').find_map(|pair| pair.strip_prefix("vault_state=")).unwrap_or_default().to_owned()
}

#[test]
fn a_vault_set_by_a_player_wakes_the_next_tick() {
    // research/captures/bds-x2ran.txt: set with the actor beside it, AnimationVaultActivate came with the set's
    // packets and the active vault a tick on.
    let mut h = ready();
    put(&mut h, VAULT, "minecraft:vault");
    h.tick_all();
    h.tick_all();
    assert_eq!(vault_state(&mut h, VAULT), "active");
}

fn connected(data: &BlockEntityData) -> Option<usize> {
    match data.nbt.decode().ok()?.value.get("connected_players")? {
        Value::List(list) => Some(list.items.len()),
        _ => Some(0),
    }
}

#[test]
fn a_cloned_vault_is_connected_to_nobody_and_shown_only_after_its_block() {
    // research/captures/bds-x2ran.txt: the clone of an active vault listed twice, then its entity twice with
    // `connected_players` empty; no entity before the block.
    let mut h = ready();
    put(&mut h, VAULT, "minecraft:vault");
    for _ in 0..25 {
        h.tick_all();
    }
    assert_eq!(vault_state(&mut h, VAULT), "active");
    let to = [5, 65, 0];
    h.remote.run(move |ctx| {
        let copies = [Copied { from: VAULT, to, block: ctx.block(VAULT).expect("loaded"), entity: ctx.block_entity(VAULT) }];
        ctx.copy_blocks(&[], &copies);
    });
    let sent = h.tick_all().to(1).to_vec();
    let at_copy = |packet: &RawPacket| all::<BlockEntityData>(std::slice::from_ref(packet)).into_iter().find(|data| [data.position.x, data.position.y, data.position.z] == to);
    let shown: Vec<(usize, Option<usize>)> = sent.iter().enumerate().filter_map(|(i, packet)| at_copy(packet).map(|data| (i, connected(&data)))).collect();
    let listing = sent.iter().position(|packet| !all::<UpdateSubchunkBlocks>(std::slice::from_ref(packet)).is_empty()).expect("listed");
    assert_eq!(shown.iter().map(|(_, players)| *players).collect::<Vec<_>>(), [Some(0), Some(0)], "{shown:?}");
    assert!(shown.iter().all(|(i, _)| *i > listing), "{shown:?} after {listing}");
}

#[test]
fn a_sensor_plays_its_power_sound_after_its_new_phase() {
    // research/captures/bds-x2drink.txt: UpdateBlock sculk_sensor_phase=1, then power.on.sculk_sensor; phase 0
    // again, then power.off.
    let mut h = ready();
    let (trapdoor, sensor) = ([2, 65, -2], [6, 65, -2]);
    put(&mut h, trapdoor, "minecraft:trapdoor");
    put(&mut h, sensor, "minecraft:sculk_sensor");
    for _ in 0..4 {
        h.tick_all();
    }
    h.receive(1, &click(trapdoor, 1, None));
    let sent: Vec<RawPacket> = (0..80).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let sound = |name: &str| sent.iter().position(|packet| all::<LevelSoundEvent>(std::slice::from_ref(packet)).iter().any(|sound| sound.sound_id == name));
    let shown = |phase: &str| {
        let id = hash(&Block::new("minecraft:sculk_sensor").with(&format!("sculk_sensor_phase={phase}")));
        sent.iter().position(|packet| block_changes(std::slice::from_ref(packet)).contains(&(sensor, id)))
    };
    let (on, active) = (sound("power.on.sculk_sensor"), shown("1"));
    assert!(on.is_some() && active < on, "{active:?} then {on:?}");
    let (off, idle) = (sound("power.off.sculk_sensor"), shown("0"));
    assert!(off.is_some() && idle < off, "{idle:?} then {off:?}");
}
