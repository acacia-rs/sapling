//! Sleeping in a bed: only at night, and a night everyone sleeps through is skipped.

mod common;

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

#[test]
fn a_night_slept_through_is_skipped() {
    let mut h = with_a_bed();
    h.remote.run(|ctx| ctx.set_time(13_000));
    h.tick(1);
    h.receive(1, &click(HEAD, 1, None));
    let sent = h.tick(1);
    assert!(lies_down(&sent));
    let counts = all::<LevelEventGeneric>(&sent);
    assert_eq!(counts.iter().map(|event| (event.event_id, event.nbt.iter().map(|tag| (tag.name.clone(), tag.value.clone())).collect::<Vec<_>>())).collect::<Vec<_>>(), [(
        9801,
        vec![("ableToSleep".into(), Value::Int(1)), ("overworldPlayerCount".into(), Value::Int(1)), ("sleepingPlayerCount".into(), Value::Int(1))]
    )]);
    let woke = (0..110).any(|_| all::<Animate>(&h.tick(1)).iter().any(|animate| animate.action_id == AnimateActionId::WakeUp));
    assert!(woke, "woken after 100 ticks");
    assert!(time(&mut h).rem_euclid(24_000) < 200, "it is morning");
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
    assert_eq!(risen, [(1, 0.5, 0.5)], "at the world's spawn once the bed is gone");
    assert!(told.iter().any(|line| line.contains("tile.bed.notValid")), "{told:?}");
}
