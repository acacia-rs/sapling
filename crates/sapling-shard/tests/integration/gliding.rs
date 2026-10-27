//! Elytra flight: started, worn, boosted and ended as BDS did (research/captures/bds-glide.txt);
//! flying into a wall as BDS's code reads (research/captures/bds-combat-findings.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{LevelSoundEvent, MovementEffect, PlayerArmorDamage, PlayerAuthInput, SetEntityData, UpdateAttributes};
use acacia_proto::types::{InputData, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, MovementEffectType, Vec3f};
use common::shooting::{ALEX, added, click_air, held};
use common::{Harness, all, input_at};
use sapling_api::{GameMode, Item, Slot};

const ROCKET: &str = "minecraft:firework_rocket";
const HIGH: [f32; 3] = [0.5, 70.0, 0.5];

/// Alex in survival 5 blocks up, with what `elytra` names on and two rockets in hand; Steve
/// beside the spawn sees it.
fn aloft(elytra: Option<&'static str>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Survival);
        assert!(ctx.set_item(ALEX, Slot::Armor(1), elytra.map(|name| Item::new(name, 1)).as_ref()));
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(ROCKET, 2))));
    });
    h.tick_all();
    h.receive(1, &input_at(HIGH));
    h.tick_all();
    h
}

/// Alex's input in the air at `HIGH` with `flags`; what each was told: Alex, then Steve.
fn fly(h: &mut Harness, flags: Vec<InputData>) -> (Vec<RawPacket>, Vec<RawPacket>) {
    h.receive(1, &PlayerAuthInput { input_data: flags, delta: Vec3f { x: 0.5, y: -0.1, z: 0.0 }, ..input_at(HIGH) });
    let sent = h.tick_all();
    (sent.to(1).to_vec(), sent.to(2).to_vec())
}

/// Whether each pose told in `sent` has the glide flag, and the box's height told with it.
fn glides(sent: &[RawPacket]) -> Vec<(bool, f32)> {
    let pose = |data: &SetEntityData| {
        let flags = data.metadata.iter().find_map(|item| if let Value::Flags(flags) = &item.value { Some(*flags) } else { None })?;
        let height = data.metadata.iter().find_map(|item| match (&item.key, &item.value) {
            (Key::BoundingboxHeight, Value::Default(Plain::Float(height))) => Some(*height),
            _ => None,
        })?;
        Some((flags.contains(MetadataFlags1::GLIDING), height))
    };
    all::<SetEntityData>(sent).iter().filter_map(pose).collect()
}

#[test]
fn a_glide_starts_in_the_air_wears_the_elytra_each_second_and_ends_on_landing() {
    let mut h = aloft(Some("minecraft:elytra"));
    let (own, seen) = fly(&mut h, vec![InputData::StartGliding]);
    assert_eq!((glides(&own), glides(&seen)), (vec![(true, 0.6)], vec![(true, 0.6)]), "told the player and its viewers");
    let mut worn = Vec::new();
    for _ in 0..40 {
        worn.extend(all::<PlayerArmorDamage>(&fly(&mut h, Vec::new()).0));
    }
    let told: Vec<(i64, i16)> = worn.iter().flat_map(|told| told.entries.iter().map(|entry| (entry.armor_slot.to_raw(), entry.damage))).collect();
    assert_eq!(told, [(2, 1), (2, 2)], "the chest's damage alone, once a second");
    h.receive(1, &PlayerAuthInput { input_data: vec![InputData::VerticalCollision], ..input_at([0.5, 64.0, 0.5]) });
    let sent = h.tick_all();
    assert_eq!((glides(sent.to(1)), glides(sent.to(2))), (vec![(false, 1.8)], vec![(false, 1.8)]), "landing ends it unasked");
}

#[test]
fn a_rocket_used_while_gliding_boosts_for_twice_its_life_and_rides_along() {
    let mut h = aloft(Some("minecraft:elytra"));
    fly(&mut h, vec![InputData::StartGliding]);
    h.receive(1, &click_air());
    let sent = h.tick_all().to(1).to_vec();
    let boost = all::<MovementEffect>(&sent);
    assert!(boost.len() == 1 && boost[0].effect_type == MovementEffectType::GLIDEBOOST && boost[0].runtime_id == 1, "{boost:?}");
    // A rocket of no Flight lasts 10 ticks and up to 5 and 6 more.
    assert!((20..=42).contains(&boost[0].effect_duration) && boost[0].effect_duration % 2 == 0, "{}", boost[0].effect_duration);
    assert_eq!(added(&sent, "minecraft:fireworks_rocket").len(), 1);
    assert_eq!(held(&mut h, Slot::Main(0)).map(|item| item.count), Some(1), "one rocket spent");
}

#[test]
fn a_glide_without_an_elytra_is_refused_and_the_client_set_right() {
    let mut h = aloft(None);
    let (own, seen) = fly(&mut h, vec![InputData::StartGliding]);
    assert_eq!((glides(&own), glides(&seen)), (vec![(false, 1.8)], vec![]));
    let mut h = aloft(Some("minecraft:elytra"));
    h.receive(1, &PlayerAuthInput { input_data: vec![InputData::StartGliding, InputData::VerticalCollision], ..input_at([0.5, 64.0, 0.5]) });
    assert_eq!(glides(h.tick_all().to(1)), [(false, 1.8)], "not on the ground");
    h.receive(1, &click_air());
    assert!(all::<MovementEffect>(h.tick_all().to(1)).is_empty(), "no boost without a glide");
}

#[test]
fn flying_into_a_wall_costs_ten_for_each_block_a_tick_of_speed_lost_less_three() {
    let mut h = aloft(Some("minecraft:elytra"));
    fly(&mut h, vec![InputData::StartGliding]);
    let (mut own, mut seen) = (Vec::new(), Vec::new());
    for across in [1.5, 1.5, 0.0, 0.0] {
        h.receive(1, &PlayerAuthInput { delta: Vec3f { x: across, y: -0.1, z: 0.0 }, ..input_at(HIGH) });
        let sent = h.tick_all();
        own.extend(sent.to(1).to_vec());
        seen.extend(sent.to(2).to_vec());
    }
    let told = all::<UpdateAttributes>(&own).into_iter().flat_map(|update| update.attributes);
    let health: Vec<f32> = told.filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect();
    assert_eq!(health, [8.0], "(1.5 - 0) * 10 - 3, once");
    let heard: Vec<String> = all::<LevelSoundEvent>(&seen).into_iter().map(|sound| sound.sound_id).collect();
    assert!(heard.iter().any(|sound| sound == "fall.big"), "{heard:?}");
}
