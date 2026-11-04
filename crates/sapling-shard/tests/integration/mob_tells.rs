//! What mobs tell their viewers beyond their moves: effect flags, an evoker's vexes re-telling
//! their prey.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, LevelSoundEvent, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all};
use sapling_api::Difficulty;

fn targets(sent: &[RawPacket], id: u64) -> Vec<i64> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata).filter(|item| item.key == Key::TargetEid).filter_map(|item| if let Value::Default(Plain::Long(target)) = item.value { Some(target) } else { None }).collect()
}

fn fire_immune(sent: &[RawPacket], id: u64) -> Vec<bool> {
    let fire_immune = acacia_proto::types::MetadataFlags1::FIRE_IMMUNE.0;
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata).filter_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0 & fire_immune != 0) } else { None }).collect()
}

#[test]
fn fire_resistance_flags_a_mob_fire_immune_and_water_breathing_does_not() {
    // research/captures/bds-t2flags.txt: bit 50 with the effect, off as it is cleared.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:cow", [3.5, 65.0, 0.5]).is_some()));
    let id = (0..3).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).find(|added| added.entity_type == "minecraft:cow").expect("shown").runtime_id;
    let cow = sapling_api::EntityId(id);
    let after = |h: &mut Harness, action: Box<dyn FnOnce(&mut sapling_api::ShardCtx<'_>) + Send>| {
        h.remote.run(action);
        (0..2).flat_map(|_| fire_immune(h.tick_all().to(1), id)).collect::<Vec<bool>>()
    };
    let breathes = after(&mut h, Box::new(move |ctx| assert!(ctx.give_entity_effect(cow, sapling_api::Effect::WaterBreathing, 400, 0))));
    assert!(breathes.is_empty(), "no flags for water breathing");
    assert_eq!(after(&mut h, Box::new(move |ctx| assert!(ctx.give_entity_effect(cow, sapling_api::Effect::FireResistance, 400, 0)))), [true]);
    assert_eq!(after(&mut h, Box::new(move |ctx| assert!(ctx.clear_entity_effects(cow)))), [false]);
}

#[test]
fn a_goat_takes_a_player_near_it_between_rams_and_lets_it_go_past_16() {
    // research/captures/bds-goat-findings.md "Target": `TargetEid`, `mob.warning` a tick on; 0 when over 16.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        let goat = ctx.spawn_entity("minecraft:goat", [0.5, 65.0, 5.5]).expect("a goat");
        assert!(ctx.give_entity_effect(goat, sapling_api::Effect::Slowness, 4000, 255));
    });
    let sent: Vec<Vec<RawPacket>> = (0..200).map(|_| h.tick_all().to(1).to_vec()).collect();
    let goat = sent.iter().flat_map(|tick| all::<AddEntity>(tick)).find(|added| added.entity_type == "minecraft:goat").expect("shown").runtime_id;
    let took = sent.iter().position(|tick| targets(tick, goat) == [1]).expect("it takes Alex");
    let warned = |tick: &[RawPacket]| all::<LevelSoundEvent>(tick).iter().any(|sound| sound.sound_id == "mob.warning" && sound.entity_type == "minecraft:goat");
    assert!(warned(&sent[took + 1]) && !warned(&sent[took]), "warned a tick on");
    h.remote.run(|ctx| ctx.teleport(sapling_api::PlayerId(1), [0.5, 65.0, -14.5]));
    let after: Vec<Vec<RawPacket>> = (0..5).map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(after.iter().any(|tick| targets(tick, goat) == [0]), "let go 20 off");
}

#[test]
fn an_evokers_vex_tells_its_prey_every_tick() {
    // research/captures/bds-evoker.txt: each tick after the take, `mob.warning` and `TargetEid` again.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        assert!(ctx.spawn_entity("minecraft:evocation_illager", [0.5, 65.0, 6.5]).is_some());
    });
    let (mut vexes, mut told) = (Vec::new(), Vec::new());
    for _ in 0..2400 {
        let sent = h.tick_all().to(1).to_vec();
        vexes.extend(all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:vex").map(|added| added.runtime_id));
        let Some(&vex) = vexes.first() else { continue };
        let warned = all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "mob.warning" && sound.entity_type == "minecraft:vex");
        told.push((targets(&sent, vex), warned));
        if told.iter().position(|(targets, _)| targets.contains(&1)).is_some_and(|first| told.len() > first + 6) {
            break;
        }
    }
    let first = told.iter().position(|(targets, _)| targets.contains(&1)).expect("the vex takes Alex");
    assert!(told.len() > first + 6 && told[first + 1..].iter().all(|(targets, warned)| targets == &[1] && *warned), "{told:?}");
}
