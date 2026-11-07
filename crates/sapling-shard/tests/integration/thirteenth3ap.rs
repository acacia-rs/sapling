//! Against the group AP drills: an allay collecting what matches its hand and bringing it to its owner
//! (research/captures/bds-ap3give.txt) or to a note block it heard (bds-ap3note.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, BlockEvent, TakeItemEntity};
use common::raising::click_on;
use common::shooting::{FEET, ready, spawn, watch};
use common::{Harness, all};
use sapling_api::{Block, GameMode, Item};

const DIRT: &str = "minecraft:dirt";

fn network_id(item: &str) -> i16 {
    sapling_data::item(&sapling_data::Stack { name: item, count: 1, damage: 0, block: None, tag: None }, None).expect("an item").network_id as i16
}

/// An allay a block from Alex, handed one of its 4 dirt; its runtime id.
fn owned_allay(h: &mut Harness) -> u64 {
    let allay = spawn(h, "minecraft:allay", [FEET[0] + 1.0, FEET[1], FEET[2]]);
    click_on(h, allay);
    watch(h, 2);
    allay
}

fn drop_at(h: &mut Harness, item: &'static str, at: [f32; 3]) {
    h.remote.run(move |ctx| assert!(ctx.drop_item(&Item::new(item, 1), at)));
}

/// What Alex was told over `n` ticks, with the tick each came in.
fn told<T: acacia_proto::Packet>(sent: &[Vec<RawPacket>]) -> Vec<(usize, T)> {
    sent.iter().enumerate().flat_map(|(at, packets)| all::<T>(packets).into_iter().map(move |packet| (at, packet))).collect()
}

/// The thrown item's velocity aims at `to` a block up from 0.6654 under where it was shown, at 0.2 across and 0.3 up.
fn aims_at(thrown: &AddItemEntity, to: [f32; 3]) -> bool {
    let feet = [thrown.position.x, thrown.position.y - 0.6654, thrown.position.z];
    let aim = [to[0] - feet[0], to[1] + 1.0 - feet[1], to[2] - feet[2]];
    let long = aim.iter().map(|c| c * c).sum::<f32>().sqrt();
    let wanted = [aim[0] / long * 0.2, aim[1] / long * 0.3, aim[2] / long * 0.2];
    let shown = [thrown.velocity.x, thrown.velocity.y, thrown.velocity.z];
    let near = (0..3).map(|axis| (to[axis] - feet[axis]).powi(2)).sum::<f32>().sqrt() < 3.0;
    near && (0..3).all(|axis| (wanted[axis] - shown[axis]).abs() < 1e-3)
}

#[test]
fn an_allay_collects_dirt_lying_near_brings_it_to_its_owners_eyes_and_leaves_cobblestone() {
    // bds-ap3give.txt: taken within 2.2, the cobblestone left; thrown within 3 of the owner's eyes, who took it.
    let mut h = ready(GameMode::Survival, &[(DIRT, 4, 0)], 0.0);
    let allay = owned_allay(&mut h);
    drop_at(&mut h, "minecraft:cobblestone", [FEET[0] - 5.0, FEET[1] + 0.5, FEET[2] + 3.0]);
    drop_at(&mut h, DIRT, [FEET[0] + 6.0, FEET[1] + 0.5, FEET[2] + 4.0]);
    let sent: Vec<Vec<RawPacket>> = (0..300).map(|_| h.tick_all().to(1).to_vec()).collect();
    let (adds, takes) = (told::<AddItemEntity>(&sent), told::<TakeItemEntity>(&sent));
    let dirt = network_id(DIRT);
    let dropped = adds.iter().find(|(_, added)| added.item.network_id == dirt).expect("the dirt dropped").1.runtime_entity_id;
    let cobblestone = adds.iter().find(|(_, added)| added.item.network_id != dirt).expect("the cobblestone dropped").1.runtime_entity_id;    let took = takes.iter().find(|(_, taken)| taken.runtime_entity_id == dropped).expect("the dirt taken");
    assert_eq!(u64::from(took.1.target), allay, "by the allay");
    assert!(takes.iter().all(|(_, taken)| taken.runtime_entity_id != cobblestone), "the cobblestone left");
    let thrown = adds.iter().find(|(at, added)| *at >= took.0 && added.item.network_id == dirt).expect("thrown back");
    let eyes = [FEET[0], FEET[1] + sapling_data::EYE_HEIGHT, FEET[2]];
    assert!(aims_at(&thrown.1, eyes), "{:?} {:?}", thrown.1.position, thrown.1.velocity);
    assert!(takes.iter().any(|(at, taken)| *at > thrown.0 && taken.runtime_entity_id == thrown.1.runtime_entity_id && u64::from(taken.target) != allay), "Alex took it");
}

#[test]
fn an_allay_brings_dirt_to_a_note_block_it_heard_and_takes_it_up_again_once_rested() {
    // bds-ap3note.txt: the dirt went to the played note block, not the owner; 66 ticks on the allay took it again.
    let mut h = ready(GameMode::Survival, &[(DIRT, 4, 0)], 0.0);
    let allay = owned_allay(&mut h);
    let note = [4, 65, -6];
    h.remote.run(move |ctx| {
        assert!(ctx.update_block(note, &Block::new("noteblock")));
        assert!(ctx.update_block([note[0], note[1], note[2] - 1], &Block::new("redstone_block")));
    });
    let played: Vec<RawPacket> = watch(&mut h, 3);
    assert_eq!(all::<BlockEvent>(&played).len(), 1, "the note played");
    drop_at(&mut h, DIRT, [FEET[0] + 6.0, FEET[1] + 0.5, FEET[2] + 4.0]);
    let sent: Vec<Vec<RawPacket>> = (0..300).map(|_| h.tick_all().to(1).to_vec()).collect();
    let (adds, takes) = (told::<AddItemEntity>(&sent), told::<TakeItemEntity>(&sent));
    let dirt = network_id(DIRT);
    let dropped = adds.iter().find(|(_, added)| added.item.network_id == dirt).expect("the dirt dropped").1.runtime_entity_id;
    let took = takes.iter().find(|(_, taken)| taken.runtime_entity_id == dropped).expect("the dirt taken");
    let thrown = adds.iter().find(|(at, added)| *at >= took.0 && added.item.network_id == dirt).expect("thrown");
    assert!(aims_at(&thrown.1, note.map(|c| c as f32 + 0.5)), "at the note block: {:?} {:?}", thrown.1.position, thrown.1.velocity);
    let again = takes.iter().find(|(_, taken)| taken.runtime_entity_id == thrown.1.runtime_entity_id).expect("taken up again");
    assert_eq!(u64::from(again.1.target), allay);
    assert!(again.0 >= thrown.0 + 60, "after its 3 s rest: {} then {}", thrown.0, again.0);
}
