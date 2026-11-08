//! Group C of the fourteenth wave: a bare sneak's shield bits and their echo to the sneaker, and what
//! a throw tells and when its thing goes (research/captures/bds-f14csneak.txt, bds-f14cthrow.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, Animate, LevelEvent, LevelEventEvent, LevelSoundEvent, MobArmorEquipment, MobEquipment, RemoveEntity, SetEntityData};
use acacia_proto::types::{InputData, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, WindowID};
use common::shooting::{ALEX, click_air, ready};
use common::{Harness, all, input_at};
use sapling_api::{Difficulty, GameMode, Item, Slot};

/// Bits 8 (raised) and 9 (moving) of each `FlagsExtended` told of Alex in `sent`.
fn shield_bits(sent: &[RawPacket]) -> Vec<i64> {
    let told = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == 1);
    told.flat_map(|data| data.metadata).filter_map(|item| if let Value::FlagsExtended(flags) = item.value { Some(flags.0 >> 8 & 3) } else { None }).collect()
}

fn told_pose(sent: &[RawPacket]) -> bool {
    all::<SetEntityData>(sent).iter().any(|data| data.runtime_entity_id == 1 && data.metadata.iter().any(|item| item.key == Key::BoundingboxHeight))
}

fn sneak(h: &mut Harness, data: InputData) -> [Vec<RawPacket>; 2] {
    let mut input = input_at([0.5, 65.0, 0.5]);
    input.input_data = vec![data];
    h.receive(1, &input);
    let mut sent = [Vec::new(), Vec::new()];
    for _ in 0..3 {
        let tick = h.tick_all();
        sent[0].extend(tick.to(1).iter().cloned());
        sent[1].extend(tick.to(2).iter().cloned());
    }
    sent
}

#[test]
fn a_bare_sneak_shows_the_shield_bits_to_all_and_its_pose_to_the_sneaker_and_a_shield_clears_what_lingers() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.tick_all();
    let [own, seen] = sneak(&mut h, InputData::StartSneaking);
    assert!(told_pose(&own) && told_pose(&seen), "the sneaker is told its own pose");
    assert_eq!((shield_bits(&own), shield_bits(&seen)), (vec![3], vec![3]), "raised and moving");
    let [own, seen] = sneak(&mut h, InputData::StopSneaking);
    assert!(told_pose(&own));
    assert_eq!((shield_bits(&own), shield_bits(&seen)), (vec![2], vec![2]), "moving stays");
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Offhand, Some(&Item::new("minecraft:shield", 1)))));
    let sent: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    assert_eq!(shield_bits(&sent), [0], "a shield in hand clears it");
}

/// Throws what Alex holds straight down; what Alex was told in each of the next `ticks` ticks.
pub(crate) fn thrown(item: &str, data: i16, ticks: usize) -> Vec<Vec<RawPacket>> {
    let mut h = ready(GameMode::Creative, &[(item, 1, data)], 90.0);
    h.receive(1, &click_air());
    (0..ticks).map(|_| h.tick_all().to(1).to_vec()).collect()
}

pub(crate) fn tick_of(ticks: &[Vec<RawPacket>], found: impl Fn(&[RawPacket]) -> bool) -> Option<usize> {
    ticks.iter().position(|sent| found(sent))
}

fn event(sent: &[RawPacket], event: LevelEventEvent) -> bool {
    all::<LevelEvent>(sent).iter().any(|told| told.event == event)
}

fn heard(sent: &[RawPacket], name: &str) -> Vec<LevelSoundEvent> {
    all::<LevelSoundEvent>(sent).into_iter().filter(|sound| sound.sound_id == name).collect()
}

fn gone(sent: &[RawPacket]) -> bool {
    !all::<RemoveEntity>(sent).is_empty()
}

#[test]
fn a_throw_tells_the_thrower_all_its_data_first_swings_nothing_and_its_potion_goes_the_tick_after_it_breaks() {
    let ticks = thrown("minecraft:splash_potion", 7, 8);
    let first = &ticks[0];
    let data = first.iter().position(|packet| packet.decode::<SetEntityData>().is_ok_and(|data| data.runtime_entity_id == 1 && data.metadata.len() > 10)).expect("the whole data");
    let at = first.iter().position(|packet| packet.decode::<AddEntity>().is_ok()).expect("the potion");
    assert!(data < at, "data, then the potion");
    let potion = first[at].decode::<AddEntity>().unwrap();
    assert!((potion.position.y - (65.0 + sapling_data::EYE_HEIGHT)).abs() < 1e-3, "from the eyes: {}", potion.position.y);
    let throw = heard(first, "throw");
    assert!(matches!(&throw[..], [sound] if (sound.position.y - 66.32).abs() < 1e-3 && (sound.position.z - 0.65).abs() < 1e-3 && sound.entity_type == "minecraft:player"), "{throw:?}");
    assert!(all::<Animate>(first).is_empty(), "no swing");
    let broke = tick_of(&ticks, |sent| event(sent, LevelEventEvent::ParticleSplash)).expect("broke");
    assert_eq!(tick_of(&ticks, gone), Some(broke + 1));
    assert!(!heard(&ticks[broke + 1], "glass").is_empty(), "glass with the RemoveEntity");
}

#[test]
fn a_snowball_leaves_from_under_the_eyes_and_goes_the_tick_after_its_puff_heard_reappearing() {
    let ticks = thrown("minecraft:snowball", 0, 6);
    let ball = all::<AddEntity>(&ticks[0]).into_iter().find(|added| added.entity_type == "minecraft:snowball").expect("thrown");
    assert!((ball.position.y - (64.9 + sapling_data::EYE_HEIGHT)).abs() < 1e-3, "{}", ball.position.y);
    let puffed = tick_of(&ticks, |sent| event(sent, LevelEventEvent::ParticleProjectileHit)).expect("puffed");
    assert_eq!(tick_of(&ticks, gone), Some(puffed + 1));
    if puffed + 1 == 2 {
        assert!(!heard(&ticks[2], "reappeared").is_empty(), "heard at two ticks old as it goes");
    }
}

/// Of `count` mobs of `kind` summoned on `difficulty`, how many were shown wearing armour, holding
/// something in the main hand, and in the off hand.
fn geared(kind: &'static str, count: usize, difficulty: Difficulty) -> [usize; 3] {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Spectator);
        ctx.set_difficulty(difficulty);
    });
    h.tick_all();
    let made = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let out = made.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = (0..count).map(|at| ctx.summon_as(None, kind, [2.5 + (at % 10) as f32, 65.0, 2.5 + (at / 10) as f32], 0.0, 0.0, None).expect("summoned").0).collect());
    let sent: Vec<RawPacket> = (0..5).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let ids: Vec<u64> = std::mem::take(&mut *made.lock().unwrap());
    let ours = |id: u64| ids.contains(&id);
    let worn = all::<MobArmorEquipment>(&sent).into_iter().filter(|worn| ours(worn.runtime_entity_id) && [&worn.helmet, &worn.chestplate, &worn.leggings, &worn.boots].iter().any(|item| item.network_id != 0));
    let held = |window: WindowID| {
        let held = all::<MobEquipment>(&sent).into_iter().filter(|held| ours(held.runtime_entity_id) && held.window_id == window && held.item.network_id != 0);
        held.map(|held| held.runtime_entity_id).collect::<std::collections::HashSet<_>>().len()
    };
    let worn: std::collections::HashSet<u64> = worn.map(|worn| worn.runtime_entity_id).collect();
    [worn.len(), held(WindowID::Inventory), held(WindowID::Offhand)]
}

#[test]
fn zombies_and_husks_roll_their_table_on_hard_and_wear_nothing_on_easy() {
    // bds-f14cgear.txt: of 40 on hard, 7 zombies and 6 husks wore armour, 3 and 6 held a weapon.
    let [worn, held, _] = geared("minecraft:zombie", 100, Difficulty::Hard);
    assert!((3..=40).contains(&worn) && (1..=20).contains(&held), "{worn} {held}");
    let [worn, held, _] = geared("minecraft:husk", 100, Difficulty::Hard);
    assert!((3..=40).contains(&worn) && (1..=20).contains(&held), "{worn} {held}");
    assert_eq!(geared("minecraft:zombie", 60, Difficulty::Easy), [0, 0, 0], "no chance on easy");
}

#[test]
fn an_unarmed_drowned_may_hold_a_shell_in_its_off_hand() {
    let [worn, _, off] = geared("minecraft:drowned", 100, Difficulty::Easy);
    assert!(worn == 0 && (1..=30).contains(&off), "{worn} {off}");
}

#[test]
fn a_lingering_potion_leaves_its_cloud_as_it_goes_the_tick_after_it_breaks() {
    let ticks = thrown("minecraft:lingering_potion", 7, 8);
    let broke = tick_of(&ticks, |sent| event(sent, LevelEventEvent::ParticleSplash)).expect("broke");
    let cloud = tick_of(&ticks, |sent| all::<AddEntity>(sent).iter().any(|added| added.entity_type == "minecraft:area_effect_cloud"));
    assert_eq!((cloud, tick_of(&ticks, gone)), (Some(broke + 1), Some(broke + 1)));
}
