//! Mobs that fight their own way: casters, shooters of what is not an arrow, leapers and fliers
//! (docs/research/mob-kinds.md).

mod common;

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use common::{Harness, all};
use sapling_api::Difficulty;

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
fn an_enderman_stared_at_for_5_ticks_takes_the_starer_with_stare_and_one_looked_past_does_not() {
    // research/captures/bds-stare-findings.md: a look within `1 - 0.025 / d` of its head, 2.465 up
    // it, held 5 ticks; 4.87 degrees off at 6 took it, 9.42 did not.
    let stared = |off: f32| {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.remote.run(|ctx| {
            ctx.set_difficulty(Difficulty::Easy);
            assert!(ctx.spawn_entity("minecraft:enderman", [0.5, 65.0, 6.5]).is_some());
        });
        let added = h.tick_all().to(1).to_vec();
        let enderman = all::<acacia_proto::packets::AddEntity>(&added)[0].runtime_id;
        h.remote.run(move |ctx| assert!(ctx.move_entity(sapling_api::EntityId(enderman), [0.5, 65.0, 6.5])));
        drop(h.tick_all());
        let pitch = -(65.0_f32 + 2.465 - 65.0 - 1.62).atan2(6.0).to_degrees();
        h.receive(1, &acacia_proto::packets::PlayerAuthInput { pitch, yaw: off, head_yaw: off, ..common::input_at([0.5, 65.0, 0.5]) });
        let sent: Vec<acacia_proto::RawPacket> = (0..12).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
        let targeted = all::<acacia_proto::packets::SetEntityData>(&sent).iter().any(|data| data.runtime_entity_id == enderman && data.metadata.iter().any(|item| item.key == acacia_proto::types::MetadataDictionaryItemKey::TargetEid));
        targeted && all::<acacia_proto::packets::LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "stare")
    };
    assert_eq!([stared(0.0), stared(4.0), stared(9.0)], [true, true, false]);
}

#[test]
fn a_vex_holds_an_iron_sword_charges_through_the_air_and_hits_for_5_5() {
    // research/captures/bds-evoker-findings.md: an iron sword; `CHARGE_ATTACK` and then `charge`;
    // 5.5 on easy (its 3 and the sword's 6) where it meets the player.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        assert!(ctx.spawn_entity("minecraft:vex", [0.5, 66.0, 5.5]).is_some());
    });
    let (mut sword, mut charged, mut health) = (false, None, Vec::new());
    for tick in 0..400 {
        let sent = h.tick_all().to(1).to_vec();
        sword |= all::<acacia_proto::packets::MobEquipment>(&sent).iter().any(|held| held.runtime_entity_id != 1);
        let charges = all::<acacia_proto::packets::SetEntityData>(&sent).iter().any(|data| data.metadata.iter().any(|item| matches!(item.value, acacia_proto::types::MetadataDictionaryItemValue::Flags(flags) if flags.0 & acacia_proto::types::MetadataFlags1::CHARGE_ATTACK.0 != 0)));
        charged = charged.or(charges.then_some(tick));
        health.extend(all::<acacia_proto::packets::UpdateAttributes>(&sent).iter().filter(|update| update.runtime_entity_id == 1).flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current));
        if health.contains(&14.5) {
            break;
        }
    }
    assert!(sword && charged.is_some() && health.first() == Some(&14.5), "sword {sword}, charged {charged:?}, health {health:?}");
}
