//! Group D of the fourteenth wave, mob projectiles (research/captures/bds-d3ghast.txt,
//! bds-d3ghast2.txt, bds-d3breeze.txt, bds-d3shulker.txt; creatures/{struck_back,fireballs,charges,
//! ghasts,shulkers}.rs, blocks/gusts.rs).

use crate::common;

use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, LevelSoundEvent, MoveEntityDelta, SetEntityData, SetEntityMotion};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use common::{Harness, all, asking::ask};
use sapling_api::{Block, Difficulty, Effect, GameMode};

const ALEX: sapling_api::PlayerId = sapling_api::PlayerId(1);

/// Alex alone on the platform at midnight on easy, with `mobs` spawned; their ids.
fn with(mobs: &'static [(&'static str, [f32; 3])]) -> (Harness, Vec<u64>) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let ids = ask(&mut h, move |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        mobs.iter().map(|(kind, feet)| ctx.spawn_entity(kind, *feet).expect("spawned").0).collect::<Vec<u64>>()
    });
    (h, ids)
}

fn flags_of(added: &AddEntity) -> i64 {
    added.metadata.iter().find_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0) } else { None }).unwrap_or(0)
}

fn moved(sent: &[acacia_proto::RawPacket], id: u64, at: &mut [f32; 3]) {
    for delta in all::<MoveEntityDelta>(sent).iter().filter(|delta| delta.runtime_entity_id == id) {
        for (axis, c) in [delta.x, delta.y, delta.z].into_iter().enumerate() {
            if let Some(c) = c {
                at[axis] = c;
            }
        }
    }
}

#[test]
fn a_ghast_fireball_comes_hidden_and_alight_and_a_hit_turns_it_back_at_1_a_tick_owned_by_the_hitter() {
    let (mut h, _) = with(&[("minecraft:ghast", [0.5, 65.0, 7.5])]);
    let (mut fireball, mut at, mut struck) = (None, [0.0; 3], None);
    let (mut owners, mut motions, mut burned) = (Vec::new(), Vec::new(), false);
    for tick in 0..200 {
        let sent = h.tick_all().to(1).to_vec();
        if fireball.is_none() {
            if let Some(added) = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:fireball") {
                assert!(flags_of(&added) & Flags::INVISIBLE.0 != 0, "hidden as it comes");
                let alight = all::<SetEntityData>(&sent).iter().any(|data| data.runtime_entity_id == added.runtime_id && data.metadata.iter().any(|item| matches!(item.value, Value::Flags(flags) if flags.0 & Flags::ONFIRE.0 != 0)));
                assert!(alight, "alight the tick it comes");
                assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "disappeared" && sound.entity_type == "minecraft:fireball"));
                (fireball, at) = (Some(added.runtime_id), [added.position.x, added.position.y, added.position.z]);
            }
        }
        let Some(id) = fireball else { continue };
        moved(&sent, id, &mut at);
        let Some(since) = struck else {
            // Close enough that its next step leaves it within reach, not yet on Alex.
            if (at[0] - 0.5).hypot(at[1] - 66.62).hypot(at[2] - 0.5) < 4.5 {
                h.receive(1, &common::attack(id));
                struck = Some(tick);
            }
            continue;
        };
        owners.extend(all::<SetEntityData>(&sent).iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata.iter().filter(|item| item.key == Key::OwnerEid).map(|item| item.value.clone())));
        motions.extend(all::<SetEntityMotion>(&sent).iter().filter(|motion| motion.runtime_entity_id == id).map(|motion| motion.velocity.x.hypot(motion.velocity.y).hypot(motion.velocity.z)));
        burned |= all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation);
        if tick > since + 10 {
            break;
        }
    }
    assert!(fireball.is_some(), "a fireball");
    assert_eq!(owners.first(), Some(&Value::Default(Plain::Long(1))), "the hitter owns it");
    assert!(motions.first().is_some_and(|speed| (speed - 1.0).abs() < 1e-3), "{motions:?}");
    assert!(!burned, "turned away from Alex");
}

#[test]
fn a_breeze_charge_that_meets_a_mob_bursts_there_hurting_it_and_throwing_it_on() {
    let (mut h, ids) = with(&[("minecraft:breeze", [0.5, 65.0, -6.5]), ("minecraft:zombie", [0.5, 65.0, -3.5])]);
    let zombie = ids[1];
    ask(&mut h, move |ctx| assert!(ctx.give_entity_effect(sapling_api::EntityId(zombie), Effect::Slowness, 2000, 255)));
    let mut struck = None;
    for tick in 0..100 {
        let sent = h.tick_all().to(1).to_vec();
        if all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == zombie && event.event_id == EntityEventEventId::HurtAnimation && event.data == 3) {
            let burst = all::<LevelEvent>(&sent).iter().any(|event| event.event == LevelEventEvent::ParticleBreezeWindExplosion);
            let thrown: Vec<[f32; 3]> = all::<SetEntityMotion>(&sent).iter().filter(|motion| motion.runtime_entity_id == zombie).map(|motion| [motion.velocity.x, motion.velocity.y, motion.velocity.z]).collect();
            struck = Some((tick, burst, thrown));
            break;
        }
    }
    let (_, burst, thrown) = struck.expect("the zombie was struck");
    assert!(burst, "burst where it met the zombie");
    // Thrown 0.4 away from the breeze and up, and by the burst on top (bds-d3breeze.txt: +0.36 up, -0.32 along).
    assert!(thrown.first().is_some_and(|first| first[1] > 0.6 && first[2] > 0.5), "{thrown:?}");
    let after = h.tick_all().to(1).to_vec();
    assert!(all::<LevelSoundEvent>(&after).iter().any(|sound| sound.sound_id == "breeze_wind_charge.burst"));
}

#[test]
fn a_breeze_charge_burst_swings_a_lever_and_a_wooden_door_but_not_an_iron_door() {
    let (mut h, _) = with(&[]);
    let door = |open: u8| format!("door_hinge_bit=0,minecraft:cardinal_direction=south,open_bit={open},upper_block_bit=0");
    let (wooden, iron) = (door(0), door(0));
    ask(&mut h, move |ctx| {
        assert!(ctx.update_block([2, 65, 0], &Block::new("minecraft:lever").with("lever_direction=up_north_south,open_bit=0")));
        assert!(ctx.update_block([-1, 65, 1], &Block::new("minecraft:wooden_door").with(&wooden)));
        assert!(ctx.update_block([2, 65, 2], &Block::new("minecraft:iron_door").with(&iron)));
        ctx.spawn_entity("minecraft:breeze", [0.5, 65.0, -6.5]).expect("a breeze");
    });
    let mut heard = Vec::new();
    for _ in 0..100 {
        let sent = h.tick_all().to(1).to_vec();
        if all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "breeze_wind_charge.burst") {
            heard = all::<LevelSoundEvent>(&sent).iter().map(|sound| (sound.sound_id.clone(), [sound.position.x, sound.position.y, sound.position.z])).collect();
            break;
        }
    }
    let swung = ask(&mut h, |ctx| [[2, 65, 0], [-1, 65, 1], [2, 65, 2]].map(|pos| ctx.block(pos).is_some_and(|block| block.properties.contains("open_bit=1"))));
    assert_eq!(swung, [true, true, false], "{heard:?}");
    assert!(heard.iter().any(|(name, at)| name == "button.click_on" && *at == [2.5, 65.5, 0.5]), "{heard:?}");
}

#[test]
fn a_ghast_held_by_slowness_255_hangs_still() {
    let (mut h, ids) = with(&[("minecraft:ghast", [0.5, 78.0, -4.5])]);
    let ghast = ids[0];
    ask(&mut h, move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.give_entity_effect(sapling_api::EntityId(ghast), Effect::Slowness, 2000, 255));
    });
    // What it drifts from the push of its first unslowed ticks dies out within 40.
    let (mut at, mut settled) = ([0.5, 78.0, -4.5], [0.0; 3]);
    for tick in 0..160 {
        let sent = h.tick_all().to(1).to_vec();
        moved(&sent, ghast, &mut at);
        if tick == 40 {
            settled = at;
        }
    }
    assert!((0..3).all(|axis| (at[axis] - settled[axis]).abs() < 0.02), "{settled:?} to {at:?}");
}

#[test]
fn an_idle_shulker_peeks_open_30() {
    let (mut h, _) = with(&[("minecraft:shulker", [0.5, 65.0, 6.5])]);
    ask(&mut h, |ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    let mut peeks = Vec::new();
    for _ in 0..400 {
        let sent = h.tick_all().to(1).to_vec();
        peeks.extend(all::<SetEntityData>(&sent).iter().flat_map(|data| data.metadata.iter().filter(|item| item.key == Key::ShulkerPeekId).map(|item| item.value.clone())));
    }
    assert!(peeks.contains(&Value::Default(Plain::Int(30))) && peeks.contains(&Value::Default(Plain::Int(0))), "{peeks:?}");
}
