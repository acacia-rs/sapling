//! A mount's own strides and a ridden horse's rearing jump, against
//! research/captures/bds-horse-jump.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{InputData, MetadataDictionaryItemValue as Metadata, MetadataFlags1, Vec2f};
use common::stored::{actor_with, compound, enter_stored, floor_with};
use common::{Harness, all, config, input_at};
use sapling_world::World;

fn attribute(mob: &AddEntity, name: &str) -> (f32, f32) {
    mob.attributes.iter().find(|attribute| attribute.name == name).map(|attribute| (attribute.value, attribute.max)).expect(name)
}

#[test]
fn each_spawned_horse_is_announced_with_its_own_health_speed_and_jump_in_its_packs_ranges() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| (0..6).for_each(|n| assert!(ctx.spawn_entity("minecraft:horse", [n as f32 * 2.0, 65.0, 3.5]).is_some())));
    let horses: Vec<AddEntity> = (0..3).flat_map(|_| all::<AddEntity>(&h.tick(1))).filter(|mob| mob.entity_type == "minecraft:horse").collect();
    assert_eq!(horses.len(), 6);
    for horse in &horses {
        let ((health, max), (speed, _), (jump, _)) = (attribute(horse, "minecraft:health"), attribute(horse, "minecraft:movement"), attribute(horse, "minecraft:horse.jump_strength"));
        assert!(health == max && health.fract() == 0.0 && (15.0..=30.0).contains(&health), "{health} of {max}");
        assert!((0.1125..=0.3375).contains(&speed) && (0.4..=1.0).contains(&jump), "{speed} {jump}");
    }
    let jumps: Vec<f32> = horses.iter().map(|horse| attribute(horse, "minecraft:horse.jump_strength").0).collect();
    assert!(jumps.windows(2).any(|pair| pair[0] != pair[1]), "rolled each: {jumps:?}");
}

fn kept(name: &str, value: f32, max: f32) -> Value {
    compound([("Base", Value::Float(value)), ("Current", Value::Float(value)), ("Max", Value::Float(max)), ("Name", Value::String(name.into()))])
}

/// A tamed, saddled horse kept with 22 health, speed 0.3 and jump strength 0.9; Alex has come.
fn stored_horse() -> (Harness, Vec<RawPacket>) {
    let attributes = vec![kept("minecraft:health", 22.0, 22.0), kept("minecraft:movement", 0.3, f32::MAX), kept("minecraft:horse.jump_strength", 0.9, f32::MAX)];
    let own = vec![("IsTamed", Value::Byte(1)), ("Saddled", Value::Byte(1)), ("Attributes", Value::List(List { tag: 10, items: attributes }))];
    let horse = actor_with(-5, "minecraft:horse", [2.5, 64.0, 0.5], own);
    let mut h = Harness::with(config(), World::with_storage(floor_with(horse)));
    let sent = enter_stored(&mut h, 1);
    (h, sent)
}

#[test]
fn a_kept_horse_is_announced_with_the_strides_it_was_kept_with() {
    let (_, sent) = stored_horse();
    let horse = all::<AddEntity>(&sent).into_iter().find(|mob| mob.entity_type == "minecraft:horse").expect("the horse");
    assert_eq!([attribute(&horse, "minecraft:health"), attribute(&horse, "minecraft:horse.jump_strength")], [(22.0, 22.0), (0.9, f32::MAX)]);
}

fn rearing(sent: &[RawPacket], id: u64) -> Vec<bool> {
    let flags = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata);
    flags.filter_map(|item| if let Metadata::Flags(flags) = item.value { Some(flags.contains(MetadataFlags1::REARING)) } else { None }).collect()
}

#[test]
fn a_ridden_horse_rears_from_its_jump_until_it_lands() {
    let (mut h, sent) = stored_horse();
    let horse = all::<AddEntity>(&sent).into_iter().find(|mob| mob.entity_type == "minecraft:horse").expect("the horse").runtime_id;
    h.receive(1, &crate::riding::interact(horse));
    h.tick(1);
    let mut seen = Vec::new();
    for y in [64.0, 64.6, 65.1, 64.9, 64.3, 64.0, 64.0] {
        let mut driving = input_at([2.5, y, 0.5]);
        driving.position.y = y;
        (driving.input_data, driving.vehicle_rotation) = (vec![InputData::ClientPredictedVehicle], Some(Vec2f { x: 0.0, z: 0.0 }));
        h.receive(1, &driving);
        seen.push(rearing(&h.tick(1), horse));
    }
    // Down at 64 it has landed; it stops rearing once the height holds (see rearing.rs's TODO).
    assert_eq!(seen, [vec![], vec![true], vec![], vec![], vec![], vec![], vec![false]]);
}
