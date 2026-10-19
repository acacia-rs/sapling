//! Daylight: what the sun burns and whom it keeps from hunting
//! (research/captures/bds-sun.txt, bds-spiders.txt).

mod common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use common::{Harness, all};
use sapling_api::{Block, Difficulty, GameMode, PlayerId};

#[test]
fn the_sun_burns_a_zombie_under_the_sky_and_not_a_creeper_or_one_under_a_roof() {
    // research/captures/bds-sun.txt: 1 a second from the tick it stands in the sun.
    let burnt = |kind: &'static str, time: i64, roofed: bool| {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.remote.run(move |ctx| {
            ctx.set_game_mode(PlayerId(1), GameMode::Creative);
            ctx.set_time(time);
            if roofed {
                assert!(ctx.set_block([4, 68, 0], &Block::new("stone")));
            }
            assert!(ctx.spawn_entity(kind, [4.5, 65.0, 0.5]).is_some());
        });
        let sent: Vec<RawPacket> = (0..65).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
        all::<EntityEvent>(&sent).iter().filter(|event| event.event_id == EntityEventEventId::HurtAnimation && event.data == 7).count()
    };
    assert_eq!([burnt("minecraft:zombie", 6000, false), burnt("minecraft:zombie", 6000, true), burnt("minecraft:zombie", 13000, false), burnt("minecraft:creeper", 6000, false)], [4, 0, 0, 0]);
}

#[test]
fn a_spider_in_the_sun_leaves_a_player_be_and_one_under_a_roof_does_not() {
    // research/captures/bds-spiders.txt.
    let bitten = |roofed: bool| {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.remote.run(move |ctx| {
            ctx.set_difficulty(Difficulty::Easy);
            ctx.set_time(6000);
            assert!(ctx.spawn_entity("minecraft:spider", [0.5, 65.0, 4.5]).is_some());
            if roofed {
                (0..=5).for_each(|z| assert!(ctx.set_block([0, 69, z], &Block::new("stone"))));
            }
        });
        let sent: Vec<RawPacket> = (0..80).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
        all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation)
    };
    assert_eq!([bitten(false), bitten(true)], [false, true]);
}

#[test]
fn a_drowned_an_endermite_a_piglin_brute_and_a_ravager_hit_for_what_bds_s_did() {
    // research/captures/bds-kinds2.txt, on easy: 2.5, 2, 6 and 7 of a player's 20; bds-provoke2.txt: a piglin's 5.5.
    for (kind, left) in [("minecraft:drowned", 17.5), ("minecraft:endermite", 18.0), ("minecraft:piglin_brute", 14.0), ("minecraft:ravager", 13.0), ("minecraft:piglin", 14.5)] {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.remote.run(move |ctx| {
            ctx.set_difficulty(Difficulty::Easy);
            ctx.set_time(18000);
            assert!(ctx.spawn_entity(kind, [0.5, 65.0, 4.5]).is_some(), "{kind}");
        });
        let sent: Vec<RawPacket> = (0..80).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
        let health: Vec<f32> = all::<acacia_proto::packets::UpdateAttributes>(&sent).iter().filter(|update| update.runtime_entity_id == 1).flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect();
        assert_eq!(health.first(), Some(&left), "{kind}: {health:?}");
    }
}

#[test]
fn a_witch_throws_poison_at_a_healthy_player_then_harming_each_three_seconds() {
    // research/captures/bds-witch.txt: 25 then 23, 60 ticks apart; poison in full, harming 6.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        assert!(ctx.spawn_entity("minecraft:witch", [0.5, 65.0, 6.5]).is_some());
    });
    let (mut thrown, mut effects, mut health) = (Vec::new(), Vec::new(), Vec::new());
    for tick in 0..140 {
        let sent = h.tick_all().to(1).to_vec();
        for added in all::<acacia_proto::packets::AddEntity>(&sent).into_iter().filter(|added| added.entity_type == "minecraft:splash_potion") {
            let potion = added.metadata.iter().find_map(|item| match (&item.key, &item.value) {
                (acacia_proto::types::MetadataDictionaryItemKey::PotionAuxValue, acacia_proto::types::MetadataDictionaryItemValue::Default(acacia_proto::types::MetadataDictionaryItemValueDefault::Short(data))) => Some(*data),
                _ => None,
            });
            thrown.push((tick, potion));
        }
        effects.extend(all::<acacia_proto::packets::MobEffect>(&sent).iter().filter(|effect| effect.runtime_entity_id == 1 && effect.event_id == acacia_proto::packets::MobEffectEventId::Add).map(|effect| (effect.effect_id, effect.duration)));
        health.extend(all::<acacia_proto::packets::UpdateAttributes>(&sent).iter().filter(|update| update.runtime_entity_id == 1).flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current));
    }
    assert!(thrown.len() == 2 && thrown[0].1 == Some(25) && thrown[1].1 == Some(23) && thrown[1].0 - thrown[0].0 == 60 && (60..80).contains(&thrown[0].0), "{thrown:?}");
    assert!(effects.contains(&(19, 900)), "{effects:?}");
    assert!(health.windows(2).any(|pair| (pair[0] - pair[1] - 6.0).abs() < 1e-3), "{health:?}");
}

#[test]
fn a_goat_waits_then_runs_off_stands_and_rams_a_player_for_2_and_a_long_push() {
    // research/captures/bds-goat-findings.md: 30 to 300 s, pre_ram, 22 ticks standing, then the
    // charge; 2 on easy, a hit's throw and then half of it and the push.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        assert!(ctx.spawn_entity("minecraft:goat", [0.5, 65.0, 6.5]).is_some());
    });
    let (mut warned, mut hurt, mut throws) = (None, None, Vec::new());
    for tick in 0..6400 {
        let sent = h.tick_all().to(1).to_vec();
        let sounds = all::<acacia_proto::packets::LevelSoundEvent>(&sent);
        warned = warned.or(sounds.iter().any(|sound| sound.sound_id == "pre_ram").then_some(tick));
        let health: Vec<f32> = all::<acacia_proto::packets::UpdateAttributes>(&sent).iter().filter(|update| update.runtime_entity_id == 1).flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect();
        if health.contains(&18.0) {
            hurt = Some(tick);
            throws = all::<acacia_proto::packets::SetEntityMotion>(&sent).iter().filter(|motion| motion.runtime_entity_id == 1).map(|motion| [motion.velocity.x, motion.velocity.y, motion.velocity.z]).collect();
            break;
        }
    }
    let (Some(warned), Some(hurt)) = (warned, hurt) else { panic!("no ram: pre_ram {warned:?}, hurt {hurt:?}") };
    assert!(warned >= 600 && (20..40).contains(&(hurt - warned)), "pre_ram at {warned}, the ram at {hurt}");
    assert!(throws.len() == 2 && (throws[0][1] - 0.4).abs() < 1e-4 && throws[1][2].abs() > 1.0, "{throws:?}");
}

#[test]
fn a_hurt_witch_drinks_healing_in_33_ticks_and_mends() {
    // research/captures/bds-witch-drinks.txt: the potion (data 21) shown in its hand, gone 33 ticks
    // on, its health up; a check a second at 0.05.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        assert!(ctx.spawn_entity("minecraft:witch", [0.5, 65.0, 2.5]).is_some());
    });
    let added = h.tick_all().to(1).to_vec();
    let witch = all::<acacia_proto::packets::AddEntity>(&added)[0].runtime_id;
    h.receive(1, &common::attack(witch));
    let (mut drank, mut emptied, mut health) = (None, None, Vec::new());
    for tick in 0..2400 {
        let sent = h.tick_all().to(1).to_vec();
        for held in all::<acacia_proto::packets::MobEquipment>(&sent).into_iter().filter(|held| held.runtime_entity_id == witch) {
            match held.item.metadata {
                21 => drank = drank.or(Some(tick)),
                _ if drank.is_some() => emptied = emptied.or(Some(tick)),
                _ => {}
            }
        }
        health.extend(all::<acacia_proto::packets::UpdateAttributes>(&sent).iter().filter(|update| update.runtime_entity_id == witch).flat_map(|update| update.attributes.clone()).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current));
        if emptied.is_some() {
            break;
        }
    }
    let (Some(drank), Some(emptied)) = (drank, emptied) else { panic!("drank {drank:?}, emptied {emptied:?}") };
    assert!(emptied - drank == 33 && health.windows(2).any(|pair| pair[1] > pair[0]), "drank {drank}, emptied {emptied}, health {health:?}");
}

#[test]
fn a_goat_rams_a_cow_when_no_player_is_there_to_ram() {
    // research/captures/bds-goat-cows-findings.md: the cow takes the ram (`HurtAnimation` 30).
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        // Off the player's line: one met first takes the ram, as BDS's watcher did.
        assert!(ctx.spawn_entity("minecraft:goat", [4.5, 65.0, -5.5]).is_some());
        assert!(ctx.spawn_entity("minecraft:cow", [4.5, 65.0, 1.5]).is_some());
    });
    let added = h.tick_all().to(1).to_vec();
    let cow = all::<acacia_proto::packets::AddEntity>(&added).iter().find(|added| added.entity_type == "minecraft:cow").expect("the cow").runtime_id;
    let rammed = (0..6400).any(|_| all::<EntityEvent>(h.tick_all().to(1)).iter().any(|event| event.runtime_entity_id == cow && event.event_id == EntityEventEventId::HurtAnimation && event.data == 30));
    assert!(rammed, "the goat rams the cow");
}

#[test]
fn a_blaze_shoots_three_small_fireballs_six_ticks_apart_that_burn_whom_they_hit() {
    // research/captures/bds-blaze.txt: the first 83 ticks after it took its prey; bds-kinds2.txt: 5 and fire.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        assert!(ctx.spawn_entity("minecraft:blaze", [0.5, 65.0, 6.5]).is_some());
    });
    let mut shots = Vec::new();
    let mut hurt = Vec::new();
    for tick in 0..160 {
        let sent = h.tick_all().to(1).to_vec();
        shots.extend(all::<acacia_proto::packets::AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:small_fireball").map(|_| tick));
        hurt.extend(all::<EntityEvent>(&sent).iter().filter(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation).map(|event| event.data));
    }
    assert!(shots.len() == 3 && shots[1] - shots[0] == 6 && shots[2] - shots[1] == 6 && (80..100).contains(&shots[0]), "{shots:?}");
    assert!(hurt.first() == Some(&3) && hurt.contains(&7), "a fireball's hit, then fire's own: {hurt:?}");
}
