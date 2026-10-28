//! Sleeping in a bed: only at night, and a night everyone sleeps through is skipped.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::nbt::Value;
use acacia_proto::packets::{Animate, AnimateActionId, LevelEventGeneric, SetEntityData, SetSpawnPosition, Text, TextContent};
use acacia_proto::types::MetadataDictionaryItemKey as Key;
use common::{Harness, all, click};
use sapling_api::{Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
/// Alex looks south and places the bed on the platform: its head lies one further south.
const FOOT: [i32; 3] = [1, 65, 0];
const HEAD: [i32; 3] = [1, 65, 1];

fn with_a_bed() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:bed", 1)))));
    h.tick(1);
    h.receive(1, &click([1, 64, 0], 1, None));
    h.tick(1);
    h
}

fn said(packets: &[acacia_proto::RawPacket]) -> Vec<String> {
    all::<Text>(packets).into_iter().filter_map(|text| if let TextContent::Json(json) = text.content { Some(json.message) } else { None }).collect()
}

fn lies_down(packets: &[acacia_proto::RawPacket]) -> bool {
    all::<SetEntityData>(packets).iter().any(|data| data.metadata.iter().any(|item| item.key == Key::PlayerBedPosition))
}

fn time(h: &mut Harness) -> i64 {
    let seen = Arc::new(Mutex::new(0));
    let out = seen.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.time());
    h.tick(1);
    *seen.lock().unwrap()
}

#[test]
fn a_bed_by_day_only_moves_the_spawn() {
    let mut h = with_a_bed();
    h.remote.run(|ctx| ctx.set_time(1000));
    h.tick(1);
    h.receive(1, &click(FOOT, 1, None));
    let sent = h.tick(1);
    let spawn = all::<SetSpawnPosition>(&sent);
    assert_eq!(spawn.iter().map(|spawn| [spawn.world_position.x, spawn.world_position.y, spawn.world_position.z]).collect::<Vec<_>>(), [HEAD]);
    let lines = said(&sent);
    assert!(lines.iter().any(|line| line.contains("tile.bed.respawnSet")) && lines.iter().any(|line| line.contains("tile.bed.noSleep")), "{lines:?}");
    assert!(!lies_down(&sent));
}

/// research/captures/bds-wake.txt: lying down corrects the client's prediction; a teleport wakes
/// it where it was sent.
#[test]
fn a_sleeper_is_put_in_its_bed_and_a_teleport_gets_it_up_where_it_was_sent() {
    use acacia_proto::packets::CorrectPlayerMovePrediction;
    let corrected = |sent: &[acacia_proto::RawPacket]| all::<CorrectPlayerMovePrediction>(sent).iter().map(|fix| ([fix.position.x, fix.position.y, fix.position.z], fix.delta.y)).collect::<Vec<_>>();
    let mut h = with_a_bed();
    h.remote.run(|ctx| ctx.set_time(13_000));
    h.tick(1);
    h.receive(1, &click(HEAD, 1, None));
    let lay = h.tick(1);
    assert_eq!(corrected(&lay), [([1.5, 65.90625, 1.5], 0.0)]);
    let width = |sent: &[acacia_proto::RawPacket]| all::<SetEntityData>(sent).iter().flat_map(|data| data.metadata.clone()).filter(|item| item.key == Key::BoundingboxWidth).map(|item| format!("{:?}", item.value)).collect::<Vec<_>>();
    assert_eq!(width(&lay), ["Default(Float(0.2))"], "a sleeper's box is small");
    let extended = |sent: &[acacia_proto::RawPacket]| all::<SetEntityData>(sent).iter().flat_map(|data| data.metadata.clone()).filter(|item| item.key == Key::FlagsExtended).map(|item| format!("{:?}", item.value)).collect::<Vec<_>>();
    assert_eq!(extended(&lay), ["FlagsExtended(MetadataFlags2(35184405647360))"], "BDS's player bits and the sleeper's");
    h.remote.run(|ctx| ctx.teleport(ALEX, [5.5, 65.0, 5.5]));
    let sent = h.tick(1);
    assert_eq!(extended(&sent), ["FlagsExtended(MetadataFlags2(35184405643264))"]);
    assert!(all::<Animate>(&sent).iter().any(|animate| animate.action_id == AnimateActionId::WakeUp));
    let moved = all::<acacia_proto::packets::MovePlayer>(&sent);
    assert_eq!(moved.iter().map(|to| [to.position.x, to.position.y, to.position.z]).collect::<Vec<_>>(), [[5.5, 65.0 + sapling_data::EYE_HEIGHT, 5.5]]);
    assert!(corrected(&sent).is_empty());
    assert_eq!(width(&sent), ["Default(Float(0.6))"]);
}

/// research/captures/bds-rest.txt: a zombie 6 blocks from the bed keeps the clicker up, one 9
/// off does not; a storm lets it sleep by day.
#[test]
fn a_monster_near_keeps_a_sleeper_up_and_a_storm_lets_it_sleep_by_day() {
    static NEAR: Mutex<Option<sapling_api::EntityId>> = Mutex::new(None);
    let mut h = with_a_bed();
    h.remote.run(|ctx| {
        ctx.set_time(18_000);
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        assert!(ctx.set_game_rule("sapling:mobAi", sapling_api::RuleValue::Bool(false)));
        let near = ctx.spawn_entity("minecraft:zombie", [7.5, 65.0, 0.5]).unwrap();
        *NEAR.lock().unwrap() = Some(near);
    });
    h.tick(1);
    h.receive(1, &click(HEAD, 1, None));
    let sent = h.tick(1);
    assert!(said(&sent).iter().any(|line| line.contains("tile.bed.notSafe")) && !lies_down(&sent));
    h.remote.run(|ctx| {
        assert!(ctx.remove_entity(NEAR.lock().unwrap().take().unwrap()));
        assert!(ctx.spawn_entity("minecraft:zombie", [10.5, 65.0, 0.5]).is_some());
        ctx.set_time(6000);
        ctx.set_weather(sapling_api::Weather::Thunder, None);
    });
    h.tick(1);
    h.receive(1, &click(HEAD, 1, None));
    assert!(lies_down(&h.tick(1)), "in a storm, the zombie 9 off");
}

/// The night ends with `WeatherManager::stopWeather` (bdsre 0x1411a79e0): the rain stops too.
#[test]
fn a_night_slept_through_is_skipped() {
    let mut h = with_a_bed();
    h.remote.run(|ctx| {
        ctx.set_time(13_000);
        ctx.set_weather(sapling_api::Weather::Rain, None);
    });
    h.tick(1);
    h.receive(1, &click(HEAD, 1, None));
    let sent = h.tick(1);
    assert!(lies_down(&sent));
    let counts = all::<LevelEventGeneric>(&sent);
    // 9801 zigzagged, as BDS sends it; acacia-proto reads the field unzigzagged.
    assert_eq!(counts.iter().map(|event| (event.event_id, event.nbt.iter().map(|tag| (tag.name.clone(), tag.value.clone())).collect::<Vec<_>>())).collect::<Vec<_>>(), [(
        19602,
        vec![("ableToSleep".into(), Value::Int(1)), ("overworldPlayerCount".into(), Value::Int(1)), ("sleepingPlayerCount".into(), Value::Int(1))]
    )]);
    let woke = (0..110).any(|_| all::<Animate>(&h.tick(1)).iter().any(|animate| animate.action_id == AnimateActionId::WakeUp));
    assert!(woke, "woken after 100 ticks");
    assert!(time(&mut h).rem_euclid(24_000) < 200, "it is morning");
    let weather = Arc::new(Mutex::new(sapling_api::Weather::Rain));
    let out = weather.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.weather());
    h.tick(1);
    assert_eq!(*weather.lock().unwrap(), sapling_api::Weather::Clear, "the rain stopped with the night");
}

/// research/captures/bds-rest.txt: a sleeper whose cell was walled while it slept stood up in the
/// next cell of BDS's search, one north.
#[test]
fn a_sleeper_whose_cell_is_walled_stands_up_in_the_next() {
    use acacia_proto::packets::CorrectPlayerMovePrediction;
    use sapling_api::Block;
    let mut h = with_a_bed();
    h.remote.run(|ctx| ctx.set_time(13_000));
    h.tick(1);
    h.receive(1, &click(HEAD, 1, None));
    assert!(lies_down(&h.tick(1)));
    h.remote.run(|ctx| assert!(ctx.set_block([0, 65, 0], &Block::new("stone")) && ctx.set_block([0, 66, 0], &Block::new("stone"))));
    let stood = (0..110).find_map(|_| all::<CorrectPlayerMovePrediction>(&h.tick(1)).first().map(|fix| [fix.position.x, fix.position.y, fix.position.z]));
    assert_eq!(stood, Some([0.5, 65.1 + sapling_data::EYE_HEIGHT, -0.5]));
}

#[test]
fn a_player_rises_by_the_bed_it_clicked_while_the_bed_stands() {
    use acacia_proto::packets::Respawn;
    use acacia_proto::types::Vec3f;
    use sapling_api::Block;
    // research/captures/bds-bedspawn.txt: the cell it stood in, then the place in it.
    let mut h = with_a_bed();
    h.remote.run(|ctx| ctx.set_time(1000));
    h.tick(1);
    h.receive(1, &click(FOOT, 1, None));
    h.tick(1);
    let rise = |h: &mut Harness| {
        h.remote.run(|ctx| assert!(ctx.kill_player(ALEX)));
        h.tick(1);
        h.receive(1, &Respawn { position: Vec3f { x: 0.0, y: 0.0, z: 0.0 }, state: 2, runtime_entity_id: 0 });
        let sent = h.tick(1);
        (all::<Respawn>(&sent).into_iter().map(|respawn| (respawn.state, respawn.position.x, respawn.position.z)).collect::<Vec<_>>(), said(&sent))
    };
    assert_eq!(rise(&mut h).0, [(0, 0.0, 0.0), (1, 0.5, 0.5)], "by the bed: named twice");
    h.remote.run(|ctx| assert!(ctx.set_block(FOOT, &Block::new("air")) && ctx.set_block(HEAD, &Block::new("air"))));
    h.tick(1);
    let (risen, told) = rise(&mut h);
    assert_eq!(risen, [(0, 0.0, 0.0), (1, 0.5, 0.5)], "the cell named once more, then the world's spawn once the bed is gone");
    assert!(told.iter().any(|line| line.contains("tile.bed.notValid")), "{told:?}");
}
