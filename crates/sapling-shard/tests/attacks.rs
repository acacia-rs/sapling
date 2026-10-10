//! A player's hits on mobs, against research/captures/bds-hit.txt.

mod common;

use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, InventoryTransaction, LevelSoundEvent, MoveEntityDelta, UpdateAttributes};
use acacia_proto::types::{
    ItemV4, ItemV4Extra, Transaction, TransactionLegacy, TransactionTransactionData, TransactionTransactionDataItemUseOnEntity, TransactionTransactionDataItemUseOnEntityActionType,
    TransactionTransactionType, Vec3f,
};
use common::{Harness, all};
use sapling_api::Block;

fn attack(id: u64) -> InventoryTransaction {
    let on = TransactionTransactionDataItemUseOnEntity {
        entity_runtime_id: id,
        action_type: TransactionTransactionDataItemUseOnEntityActionType::Attack,
        hotbar_slot: 0,
        held_item: ItemV4 { network_id: 0, count: 0, metadata: 0, has_stack_id: false, stack_id: None, block_runtime_id: 0, extra: ItemV4Extra::Default(None) },
        player_pos: Vec3f { x: 0.5, y: 65.0 + sapling_data::EYE_HEIGHT, z: 0.5 },
        click_pos: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
    };
    let legacy = TransactionLegacy { legacy_request_id: 0, legacy_transactions: None };
    InventoryTransaction { transaction: Transaction { legacy, transaction_type: TransactionTransactionType::ItemUseOnEntity, actions: Vec::new(), transaction_data: TransactionTransactionData::ItemUseOnEntity(on) } }
}

fn health(sent: &[acacia_proto::RawPacket]) -> Vec<f32> {
    named(sent, "minecraft:health")
}

fn named(sent: &[acacia_proto::RawPacket], name: &str) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == name).map(|attribute| attribute.current).collect()
}

#[test]
fn a_hit_from_too_far_is_not_answered() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:cow", [4.5, 65.0, 0.5]).is_some()));
    let cow = (0..5).flat_map(|_| all::<AddEntity>(h.tick_all().to(2))).collect::<Vec<_>>().remove(0).runtime_id;
    h.receive(1, &attack(cow));
    let sent = h.tick_all().to(2).to_vec();
    assert!(all::<EntityEvent>(&sent).is_empty() && all::<LevelSoundEvent>(&sent).is_empty() && health(&sent).is_empty());
}

#[test]
fn a_fist_costs_a_cow_one_throws_it_and_ten_kill_it_for_experience_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:cow", [2.5, 65.0, 0.5]).is_some()));
    let cow = (0..5).flat_map(|_| all::<AddEntity>(h.tick_all().to(2))).collect::<Vec<_>>().remove(0).runtime_id;

    h.receive(1, &attack(cow));
    h.receive(1, &attack(cow));
    let sent = h.tick_all().to(2).to_vec();
    let events: Vec<_> = all::<EntityEvent>(&sent).into_iter().map(|event| (event.event_id, event.data)).collect();
    assert_eq!((events, health(&sent)), (vec![(EntityEventEventId::HurtAnimation, 2)], vec![9.0]), "the second hit is within the first one's guard");
    assert_eq!(named(&sent, "minecraft:player.exhaustion"), [0.1], "the hit that landed tires the hitter");
    let swings: Vec<_> = all::<LevelSoundEvent>(&sent).into_iter().map(|sound| (sound.sound_id, sound.position.y, sound.entity_type)).collect();
    assert_eq!(swings, vec![("attack.strong".to_owned(), 65.9, "minecraft:player".to_owned()); 2], "each is heard");
    // Thrown 0.4 away and up in this tick; the next slows it as one that left the ground.
    let at = |sent: &[acacia_proto::RawPacket]| all::<MoveEntityDelta>(sent).pop().map(|moved| (moved.x.unwrap(), moved.y.unwrap()));
    let near = |(x, y): (f32, f32), wanted: (f32, f32)| (x - wanted.0).abs() < 1e-4 && (y - wanted.1).abs() < 1e-4;
    let (first, second) = (at(&sent).expect("thrown"), at(h.tick_all().to(2)).expect("flies on"));
    assert!(near(first, (2.9, 65.4)) && near(second, (3.1184, 65.7136)), "{first:?} {second:?}");

    // A second cow against a wall, so that it stays in reach.
    h.remote.run(|ctx| {
        assert!([65, 66].into_iter().all(|y| ctx.set_block([0, y, 3], &Block::new("stone"))));
        assert!(ctx.spawn_entity("minecraft:cow", [0.5, 65.0, 2.5]).is_some());
    });
    let cow = (0..5).flat_map(|_| all::<AddEntity>(h.tick_all().to(2))).collect::<Vec<_>>().remove(0).runtime_id;
    let mut dead = None;
    for hit in 1..=10 {
        (0..12).for_each(|_| drop(h.tick_all()));
        h.receive(1, &attack(cow));
        let sent = h.tick_all().to(2).to_vec();
        dead = all::<EntityEvent>(&sent).iter().any(|event| event.event_id == EntityEventEventId::DeathAnimation).then_some(hit).or(dead);
    }
    assert_eq!(dead, Some(10));
    let orbs = (0..25).flat_map(|_| all::<AddEntity>(h.tick_all().to(2))).filter(|added| added.entity_type == "minecraft:xp_orb").count();
    assert!((1..=3).contains(&orbs), "{orbs} orbs");
}
