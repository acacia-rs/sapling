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
fn an_evoker_casts_a_line_of_sixteen_fangs_a_tick_apart_that_bite_for_6_by_magic() {
    // research/captures/bds-evoker-findings.md: the first fang 21 ticks after the cast shows, 1.25
    // apart from 1.25 out, each floored; one bite for 6 a cast, thrown 0.4 from the evoker.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        assert!(ctx.spawn_entity("minecraft:evocation_illager", [0.5, 65.0, 6.5]).is_some());
    });
    let (mut cast, mut fangs, mut bites, mut throws) = (None, Vec::new(), Vec::new(), Vec::new());
    for tick in 0..100 {
        let sent = h.tick_all().to(1).to_vec();
        let spell = all::<acacia_proto::packets::SetEntityData>(&sent).iter().any(|data| data.metadata.iter().any(|item| item.key == acacia_proto::types::MetadataDictionaryItemKey::EvokerSpellCastingColor));
        cast = cast.or(spell.then_some(tick));
        fangs.extend(all::<acacia_proto::packets::AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:evocation_fang").map(|added| (tick, added.position.x, added.position.z)));
        bites.extend(all::<EntityEvent>(&sent).iter().filter(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation).map(|event| event.data));
        throws.extend(all::<acacia_proto::packets::SetEntityMotion>(&sent).iter().filter(|motion| motion.runtime_entity_id == 1).map(|motion| [motion.velocity.x, motion.velocity.y, motion.velocity.z]));
    }
    let cast = cast.expect("a cast");
    // Those past the platform's edge, z -8, have no ground and do not rise.
    let lined: Vec<(u64, f32, f32)> = (0..16).map(|i| (cast + 21 + i, 0.0, (6.5 - 1.25 * (i + 1) as f32).floor())).filter(|(_, _, z)| *z >= -8.0).collect();
    assert_eq!(fangs, lined);
    assert_eq!(bites, [14], "one bite, by magic");
    assert!(throws.len() == 1 && (throws[0][1] - 0.4).abs() < 1e-4 && (throws[0][2] + 0.4).abs() < 1e-3, "{throws:?}");
}

#[test]
fn a_shulker_opens_for_its_prey_and_its_bullet_flies_to_it_for_4_and_levitation() {
    // research/captures/bds-shulker-findings.md: open 100 with the target, the first bullet 37
    // ticks after from 0.692 up and 1 towards it, 25 ticks to a player 6 off; 4, and 200 ticks afloat.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        assert!(ctx.spawn_entity("minecraft:shulker", [0.5, 65.0, 6.5]).is_some());
    });
    let (mut opened, mut fired, mut hit, mut lifted) = (None, Vec::new(), None, Vec::new());
    for tick in 0..120 {
        let sent = h.tick_all().to(1).to_vec();
        let peeks = all::<acacia_proto::packets::SetEntityData>(&sent).iter().any(|data| data.metadata.iter().any(|item| item.key == acacia_proto::types::MetadataDictionaryItemKey::ShulkerPeekId));
        opened = opened.or(peeks.then_some(tick));
        fired.extend(all::<acacia_proto::packets::AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:shulker_bullet").map(|added| (tick, [added.position.x, added.position.y, added.position.z])));
        if hit.is_none() && all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation && event.data == 3) {
            hit = Some(tick);
        }
        lifted.extend(all::<acacia_proto::packets::MobEffect>(&sent).iter().filter(|effect| effect.runtime_entity_id == 1).map(|effect| (effect.effect_id, effect.duration)));
    }
    let opened = opened.expect("opened");
    let (first, from) = fired[0];
    assert!(first == opened + 37 && from.iter().zip([0.5, 65.692, 5.5]).all(|(a, b)| (a - b).abs() < 1e-4), "opened {opened}, fired {fired:?}");
    let hit = hit.expect("a hit");
    assert!((20..=30).contains(&(hit - first)), "fired {first}, hit {hit}");
    assert_eq!(lifted.first(), Some(&(24, 200)));
    assert!(fired.len() >= 2 && fired[1].0 - first == 41, "{fired:?}");
}

#[test]
fn a_breeze_faces_shoots_a_wind_charge_that_bursts_on_its_prey_and_leaps() {
    // research/captures/bds-breeze-findings.md: 16 ticks facing, the charge 1.416 up; it bursts 6
    // ticks after it meets the player, for 1, a hit's throw and then the burst's on top; a leap
    // readied 4 ticks after the shot is heard 11 after that.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        assert!(ctx.spawn_entity("minecraft:breeze", [0.5, 65.0, -5.5]).is_some());
    });
    let (mut facing, mut shot, mut hurt, mut throws, mut jumped) = (None, None, None, Vec::new(), None);
    for tick in 0..80 {
        let sent = h.tick_all().to(1).to_vec();
        let faces = all::<acacia_proto::packets::SetEntityData>(&sent).iter().any(|data| data.metadata.iter().any(|item| matches!(item.value, acacia_proto::types::MetadataDictionaryItemValue::FlagsExtended(flags) if flags.0 & (1 << 24) != 0)));
        facing = facing.or(faces.then_some(tick));
        shot = shot.or(all::<acacia_proto::packets::AddEntity>(&sent).iter().find(|added| added.entity_type == "minecraft:breeze_wind_charge_projectile").map(|added| (tick, added.position.y)));
        if hurt.is_none() && all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation && event.data == 3) {
            hurt = Some(tick);
            throws = all::<acacia_proto::packets::SetEntityMotion>(&sent).iter().filter(|motion| motion.runtime_entity_id == 1).map(|motion| [motion.velocity.x, motion.velocity.y, motion.velocity.z]).collect();
        }
        jumped = jumped.or(all::<acacia_proto::packets::LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "jump").then_some(tick));
    }
    let (facing, (shot, held)) = (facing.expect("facing"), shot.expect("a charge"));
    assert!(shot == facing + 16 && (held - 66.416).abs() < 1e-3, "facing {facing}, shot {shot} at {held}");
    let hurt = hurt.expect("a hit");
    assert!((6..20).contains(&(hurt - shot)), "shot {shot}, hurt {hurt}");
    assert!(throws.len() == 2 && (throws[0][1] - 0.4).abs() < 1e-4 && (throws[0][2] - 0.4).abs() < 1e-3 && throws[1][1] > 0.8, "{throws:?}");
    assert_eq!(jumped, Some(shot + 15));
}

#[test]
fn a_ghast_warns_shoots_a_fireball_21_ticks_after_that_hurts_6_and_burns_and_warns_again_60_on() {
    // research/captures/bds-ghast-findings.md: `warn` with its mouth open 20 ticks after taking its
    // prey, the fireball 21 after at 1.6 a tick; 6 where it meets the player, and fire.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        // High over the platform, which is all the world there is: a ghast that wandered below it shot its underside.
        assert!(ctx.spawn_entity("minecraft:ghast", [0.5, 78.0, -4.5]).is_some());
    });
    let (mut warns, mut opened, mut fired, mut hurt) = (Vec::new(), None, None, None);
    for tick in 0..160 {
        let sent = h.tick_all().to(1).to_vec();
        warns.extend(all::<acacia_proto::packets::LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id == "warn").map(|_| tick));
        let open = all::<acacia_proto::packets::SetEntityData>(&sent).iter().any(|data| data.metadata.iter().any(|item| item.key == acacia_proto::types::MetadataDictionaryItemKey::ChargeAmount && item.value == acacia_proto::types::MetadataDictionaryItemValue::Default(acacia_proto::types::MetadataDictionaryItemValueDefault::Byte(1))));
        opened = opened.or(open.then_some(tick));
        fired = fired.or(all::<acacia_proto::packets::AddEntity>(&sent).iter().find(|added| added.entity_type == "minecraft:fireball").map(|added| (tick, added.velocity.x.hypot(added.velocity.y).hypot(added.velocity.z))));
        if hurt.is_none() && all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation && event.data == 3) {
            hurt = Some(tick);
        }
    }
    let ((fired, speed), hurt) = (fired.expect("a fireball"), hurt.expect("a hit"));
    assert!(warns.len() >= 2 && opened == Some(warns[0]) && fired == warns[0] + 21 && warns[1] == fired + 60, "warns {warns:?}, opened {opened:?}, fired {fired}");
    assert!((speed - 1.6).abs() < 1e-3 && hurt > fired, "speed {speed}, hurt {hurt}");
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
