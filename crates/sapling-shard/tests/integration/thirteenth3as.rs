//! Against the group AS drills: a happy ghast harnessed, ridden, flown and got off
//! (research/captures/bds-as3ghast.txt), parrots on a player's shoulders (bds-as3parrot.txt), and an
//! allay's owner kept with the world (bds-as3allay.txt, bds-as3allay2.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, Interact, InteractActionId, LevelSoundEvent, MobArmorEquipment, PlayerAuthInput, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as MetaValue, MetadataDictionaryItemValueDefault as Plain, Vec2f};
use common::raising::{click_on, feet_of};
use common::shooting::{ALEX, FEET, ready, spawn, watch};
use common::stored::{enter_stored, floor_holding};
use common::{Harness, all, input_at};
use sapling_api::{Block, EntityId, GameMode, Item, Target};
use sapling_world::{ChunkPos, World, WorldStorage};

use crate::riding::links;

const GHAST: &str = "minecraft:happy_ghast";

fn sounds(sent: &[RawPacket]) -> Vec<String> {
    all::<LevelSoundEvent>(sent).into_iter().map(|sound| sound.sound_id.to_string()).collect()
}

/// A grown happy ghast over Alex's head, Alex holding a white harness; its runtime id.
fn ghast_over_alex() -> (Harness, u64) {
    let mut h = ready(GameMode::Creative, &[("minecraft:white_harness", 1, 0)], 0.0);
    let ghast = common::asking::ask(&mut h, |ctx| ctx.summon_as(None, GHAST, [0.5, 66.0, 4.0], 0.0, 0.0, Some("minecraft:spawn_adult"))).expect("summoned");
    watch(&mut h, 2);
    (h, ghast.0)
}

#[test]
fn a_harnessed_happy_ghast_seats_its_rider_with_its_camera_flies_where_it_looks_and_lets_it_off_on_top() {
    let (mut h, ghast) = ghast_over_alex();
    let harnessed = click_on(&mut h, ghast);
    let body = all::<MobArmorEquipment>(&harnessed).into_iter().find(|shown| shown.runtime_entity_id == ghast).expect("its body slot shown");
    assert_ne!(body.body.network_id, 0, "the harness in it");
    assert_eq!(sounds(&harnessed), ["armor.equip_generic"]);

    let seated = click_on(&mut h, ghast);
    assert_eq!(links(&seated), [(ghast as i64, 1, 1)]);
    assert_eq!(sounds(&seated), ["mount", "attach"], "attach with the first rider, as BDS");
    let seat = all::<SetEntityData>(&seated).into_iter().find(|data| data.runtime_entity_id == 1 && data.metadata.iter().any(|item| item.key == Key::RiderSeatPosition)).expect("Alex's seat");
    let keys: Vec<Key> = seat.metadata.iter().map(|item| item.key.clone()).collect();
    assert_eq!(keys, [Key::RiderSeatPosition, Key::RiderMaxRotation, Key::SeatThirdPersonCameraRadius, Key::SeatCameraRelaxDistanceSmoothing]);
    let interval = all::<SetEntityData>(&seated).into_iter().flat_map(|data| data.metadata).find(|item| item.key == Key::AmbientSoundInterval).map(|item| item.value);
    assert!(matches!(interval, Some(MetaValue::Default(Plain::Float(30.0)))), "{interval:?}");

    let start = feet_of(&mut h, ghast);
    for _ in 0..3 {
        h.receive(1, &PlayerAuthInput { move_vector: Vec2f { x: 0.0, z: 1.0 }, pitch: -90.0, ..input_at(FEET) });
        h.tick_all();
    }
    let risen = feet_of(&mut h, ghast)[1] - start[1];
    assert!(risen > 0.04 && (feet_of(&mut h, ghast)[0] - start[0]).abs() < 1e-3, "straight up, 0.016 more a tick after 0.91 of its speed: {risen}");

    h.receive(1, &Interact { action_id: InteractActionId::LeaveVehicle, target_entity_id: ghast, has_position: false, position: None });
    let left = watch(&mut h, 1);
    assert_eq!(links(&left), [(ghast as i64, 1, 0)]);
    assert!(sounds(&left).contains(&"detach".to_owned()), "{:?}", sounds(&left));
    let ghast_feet = feet_of(&mut h, ghast);
    let alex = common::asking::ask(&mut h, |ctx| ctx.position(ALEX)).expect("Alex").feet;
    // The ghast still drifts up from its flight while Alex, sending nothing, stays put.
    assert!((alex[0] - ghast_feet[0]).abs() < 0.01 && (alex[1] - ghast_feet[1] - 4.025).abs() < 0.2, "on its top at its middle: {alex:?} over {ghast_feet:?}");
}

#[test]
fn a_happy_ghast_takes_no_rider_without_its_harness_and_shears_take_the_harness_off() {
    let (mut h, ghast) = ghast_over_alex();
    click_on(&mut h, ghast);
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, sapling_api::Slot::Main(0), Some(&Item::new("minecraft:shears", 1)))));
    h.tick_all();
    let sheared = click_on(&mut h, ghast);
    assert_eq!(sounds(&sheared), ["armor.unequip_generic"]);
    assert!(all::<MobArmorEquipment>(&sheared).iter().any(|shown| shown.runtime_entity_id == ghast && shown.body.network_id == 0));
    click_on(&mut h, ghast);
    assert!(common::asking::ask(&mut h, move |ctx| ctx.riders(EntityId(ghast))).is_empty(), "no harness, no seat");
}

#[test]
fn a_tamed_parrot_gets_on_a_shoulder_and_off_when_the_player_has_its_eyes_in_water() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let parrot = common::asking::ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:parrot", [3.5, 65.0, 0.5], 0.0, 0.0, Some("minecraft:on_tame"))).expect("summoned");
    let mut on = None;
    for tick in 0..240 {
        let sent = h.tick_all().to(1).to_vec();
        if links(&sent).contains(&(1, parrot.0 as i64, 1)) {
            on = Some((tick, sent));
            break;
        }
    }
    if on.is_none() {
        let vehicle = common::asking::ask(&mut h, move |ctx| ctx.vehicle_of(&Target::Entity(parrot)));
        panic!("not on Alex's shoulder: at {:?}, riding {vehicle:?}", feet_of(&mut h, parrot.0));
    }
    let (tick, sent) = on.expect("on Alex's shoulder");
    assert!(tick >= 95, "not before its 100 ticks: {tick}");
    let seat = all::<SetEntityData>(&sent).into_iter().find(|data| data.runtime_entity_id == parrot.0).expect("its seat");
    let keys: Vec<Key> = seat.metadata.iter().map(|item| item.key.clone()).collect();
    assert_eq!(keys, [Key::RiderSeatPosition, Key::RiderRotationLocked]);
    assert_eq!(common::asking::ask(&mut h, move |ctx| ctx.vehicle_of(&Target::Entity(parrot))), Some(EntityId(1)));

    h.remote.run(|ctx| assert!(ctx.set_block([0, 66, 0], &Block::new("minecraft:water"))));
    let off = watch(&mut h, 3);
    assert!(links(&off).contains(&(1, parrot.0 as i64, 0)), "{:?}", links(&off));
    assert_eq!(common::asking::ask(&mut h, move |ctx| ctx.vehicle_of(&Target::Entity(parrot))), None);
}

/// The `OwnerNew` the first actor kept in `actors` with one names.
fn owner_new(actors: &[u8]) -> Option<i64> {
    let at = actors.windows(8).position(|window| window == b"OwnerNew")? + 8;
    Some(i64::from_le_bytes(actors.get(at..at + 8)?.try_into().ok()?))
}

#[test]
fn an_allay_s_owner_is_kept_with_the_world_and_shown_when_it_is_loaded() {
    let storage = floor_holding(Vec::new(), &[]);
    let mut h = Harness::with(common::config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Survival);
        assert!(ctx.set_item(ALEX, sapling_api::Slot::Main(0), Some(&Item::new("minecraft:dirt", 1))));
    });
    h.tick_all();
    let feet = common::asking::ask(&mut h, |ctx| ctx.position(ALEX)).expect("Alex").feet;
    let allay = spawn(&mut h, "minecraft:allay", [feet[0] + 1.0, feet[1], feet[2]]);
    click_on(&mut h, allay);
    watch(&mut h, 2);
    h.shard.save_worlds();
    let stored = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let owner = owner_new(&stored.actors).expect("an owner kept");
    assert_ne!(owner, -1, "the giver's unique id, as BDS kept it (bds-as3allay-dump.txt)");

    let mut again = Harness::with(common::config(), World::with_storage(storage));
    let mut shown = enter_stored(&mut again, 1);
    shown.extend(watch(&mut again, 2));
    let allay = all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type == "minecraft:allay").expect("the allay again").runtime_id;
    let mut items = all::<AddEntity>(&shown).into_iter().filter(|added| added.runtime_id == allay).flat_map(|added| added.metadata).collect::<Vec<_>>();
    items.extend(all::<SetEntityData>(&shown).into_iter().filter(|data| data.runtime_entity_id == allay).flat_map(|data| data.metadata));
    // Sapling's OwnerEid is the owner's runtime id, Alex's 1, where BDS's named its unique id (bds-as3allay2.txt).
    let owners: Vec<MetaValue> = items.into_iter().filter(|item| item.key == Key::OwnerEid).map(|item| item.value).collect();
    assert!(owners.iter().any(|owner| matches!(owner, MetaValue::Default(Plain::Long(1)))), "{owners:?}");
}
