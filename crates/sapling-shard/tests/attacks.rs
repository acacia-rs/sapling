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

#[test]
fn a_wolf_leaves_a_player_be_until_it_is_hit_and_then_bites_it() {
    // research/captures/bds-provoke.txt: its target the tick it is hit, 2.5 a bite on easy.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        assert!(ctx.spawn_entity("minecraft:wolf", [0.5, 65.0, 2.5]).is_some());
    });
    let before: Vec<acacia_proto::RawPacket> = (0..60).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let wolf = all::<AddEntity>(&before)[0].runtime_id;
    assert!(!health(&before).contains(&17.5), "left alone it bites nobody");
    h.receive(1, &attack(wolf));
    let after: Vec<acacia_proto::RawPacket> = (0..80).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(health(&after).contains(&17.5), "hit, it turns on its hitter: {:?}", health(&after));
    let later: Vec<f32> = (0..80).flat_map(|_| health(h.tick_all().to(1))).filter(|left| *left <= 17.5).collect();
    let bites = later.windows(2).filter(|pair| pair[0] - pair[1] > 2.0).count();
    assert!((2..=3).contains(&bites), "a bite each 31 ticks, not each 20: {later:?}");
}

#[test]
fn a_spider_in_the_sun_and_a_polar_bear_turn_on_whoever_hits_them() {
    // research/captures/bds-provoke2.txt, on easy: the spider's 2 a second, the bear's 4 each 39 ticks.
    for (kind, bite, between) in [("minecraft:spider", 2.0, 20), ("minecraft:polar_bear", 4.0, 39)] {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.remote.run(move |ctx| {
            ctx.set_difficulty(sapling_api::Difficulty::Easy);
            ctx.set_time(6000);
            assert!(ctx.spawn_entity(kind, [0.5, 65.0, 2.5]).is_some());
        });
        let before: Vec<acacia_proto::RawPacket> = (0..60).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
        let mob = all::<AddEntity>(&before)[0].runtime_id;
        assert!(health(&before).iter().all(|left| *left == 20.0), "{kind} left alone bites nobody");
        h.receive(1, &attack(mob));
        let mut hurt = Vec::new();
        for tick in 0..100 {
            let own = all::<UpdateAttributes>(h.tick_all().to(1)).into_iter().filter(|update| update.runtime_entity_id == 1).flat_map(|update| update.attributes);
            hurt.extend(own.filter(|attribute| attribute.name == "minecraft:health").map(|attribute| (tick, attribute.current)));
        }
        let second = hurt.iter().find(|(_, left)| (*left - (20.0 - 2.0 * bite)).abs() < 1e-3);
        assert!(hurt.first().is_some_and(|(_, left)| *left == 20.0 - bite) && second.is_some_and(|(tick, _)| tick - hurt[0].0 == between), "{kind}: {hurt:?}");
    }
}

#[test]
fn a_llama_hit_spits_once_for_1_and_throws_nobody() {
    // research/captures/bds-provoke2.txt: the spit 2.07 s after the hit, 20 to 19, no SetEntityMotion; let go 3.95 s after.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        assert!(ctx.spawn_entity("minecraft:llama", [0.5, 65.0, 3.5]).is_some());
    });
    let llama = (0..5).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).collect::<Vec<_>>().remove(0).runtime_id;
    h.receive(1, &attack(llama));
    let (mut spat, mut hurt, mut thrown) = (Vec::new(), Vec::new(), 0);
    for tick in 0..160 {
        let sent = h.tick_all().to(1).to_vec();
        spat.extend(all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:llama_spit").map(|_| tick));
        hurt.extend(all::<UpdateAttributes>(&sent).into_iter().filter(|update| update.runtime_entity_id == 1).flat_map(|update| update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current));
        thrown += all::<acacia_proto::packets::SetEntityMotion>(&sent).iter().filter(|motion| motion.runtime_entity_id == 1).count();
    }
    assert!(spat.len() == 1 && (38..=44).contains(&spat[0]), "{spat:?}");
    assert_eq!((hurt.first(), thrown), (Some(&19.0), 0), "{hurt:?}");
}

#[test]
fn an_iron_golem_hit_hits_back_for_7_to_21_and_throws_its_hitter_up() {
    // research/captures/bds-golem.txt: on easy half and one more, 4.5 to 11.5; thrown up at 0.4288.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        assert!(ctx.spawn_entity("minecraft:iron_golem", [0.5, 65.0, 2.5]).is_some());
    });
    let before: Vec<acacia_proto::RawPacket> = (0..40).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let golem = all::<AddEntity>(&before)[0].runtime_id;
    h.receive(1, &attack(golem));
    let after: Vec<acacia_proto::RawPacket> = (0..60).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let left: Vec<f32> = health(&after).into_iter().filter(|left| *left < 20.0).collect();
    let thrown: Vec<f32> = all::<acacia_proto::packets::SetEntityMotion>(&after).iter().filter(|thrown| thrown.runtime_entity_id == 1).map(|thrown| thrown.velocity.y).collect();
    assert!(left.first().is_some_and(|left| (8.5..=15.5).contains(left) && (left * 2.0).fract() == 0.0) && thrown.first() == Some(&0.4288), "{left:?} {thrown:?}");
}

#[test]
fn strength_adds_to_a_fists_hit_and_weakness_takes_most_of_it() {
    // research/captures/bds-fx.txt: of a cow's 10 a fist left 6.01 at strength's second level, 9.7 weak.
    for (effect, level, left) in [(sapling_api::Effect::Strength, 1, 6.01), (sapling_api::Effect::Strength, 0, 7.7), (sapling_api::Effect::Weakness, 0, 9.7)] {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.enter(2, "Steve");
        h.remote.run(move |ctx| {
            assert!(ctx.spawn_entity("minecraft:cow", [2.5, 65.0, 0.5]).is_some());
            assert!(ctx.give_effect(sapling_api::PlayerId(1), effect, 400, level, true));
        });
        let cow = (0..3).flat_map(|_| all::<AddEntity>(h.tick_all().to(2))).collect::<Vec<_>>().remove(0).runtime_id;
        h.receive(1, &attack(cow));
        let now = health(h.tick_all().to(2));
        assert!(now.len() == 1 && (now[0] - left).abs() < 1e-3, "{effect:?} {level}: {now:?}");
    }
}

#[test]
fn a_player_hits_another_for_what_it_holds_and_not_with_pvp_off_or_one_in_creative() {
    // research/captures/bds-pvp.txt: a fist's 1 on easy, a guard for the second at once, thrown
    // 0.4 up; nothing at all with the rule off; a creative player hears it and loses nothing.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| ctx.set_difficulty(sapling_api::Difficulty::Easy));
    h.tick_all();
    let steve = 2;
    h.receive(1, &attack(steve));
    h.receive(1, &attack(steve));
    let sent = h.tick_all().to(2).to_vec();
    let thrown: Vec<f32> = all::<acacia_proto::packets::SetEntityMotion>(&sent).iter().map(|thrown| thrown.velocity.y).collect();
    assert_eq!((health(&sent), thrown), (vec![19.0], vec![0.4]));
    (0..12).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert!(ctx.set_game_rule("pvp", sapling_api::RuleValue::Bool(false))));
    h.tick_all();
    h.receive(1, &attack(steve));
    let sent = h.tick_all().to(2).to_vec();
    assert!(health(&sent).is_empty() && all::<LevelSoundEvent>(&sent).is_empty(), "with pvp off nothing lands or sounds");
    h.remote.run(|ctx| {
        ctx.set_game_rule("pvp", sapling_api::RuleValue::Bool(true));
        ctx.set_game_mode(sapling_api::PlayerId(2), sapling_api::GameMode::Creative);
    });
    h.tick_all();
    h.receive(1, &attack(steve));
    let sent = h.tick_all().to(2).to_vec();
    assert!(health(&sent).is_empty() && all::<LevelSoundEvent>(&sent).len() == 1, "a creative player hears the swing");
}
