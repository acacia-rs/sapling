//! What a world brings besides its blocks: the time of day, the difficulty, and its stored mobs.

mod common;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use acacia_proto::RawPacket;
use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Value};
use acacia_proto::packets::{
    AddEntity, PlayStatus, RemoveEntity, ResourcePackClientResponseResponseStatus as PackStatus, SetLocalPlayerAsInitialized, StartGame, SyncWorldClocks,
    SyncWorldClocksContent as Clocks,
};
use acacia_proto::types::GameRuleI32Value;
use bytes::BytesMut;
use common::{Harness, all, config, pack_response, platform};
use sapling_api::Difficulty;
use sapling_shard::ShardConfig;
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, Level, MemoryStorage, StoredColumn, World, WorldStorage};

/// The clock packets among `sent` as (time, paused), registry or state alike.
fn clocks(sent: &[RawPacket]) -> Vec<(i32, bool)> {
    let read = |packet: SyncWorldClocks| -> Vec<(i32, bool)> {
        match packet.content {
        Clocks::InitializeRegistry(registry) => registry.clocks.iter().map(|clock| (clock.time, clock.paused)).collect(),
        Clocks::SyncState(state) => state.sync_states.iter().map(|clock| (clock.time, clock.paused)).collect(),
            other => panic!("{other:?}"),
        }
    };
    all::<SyncWorldClocks>(sent).into_iter().flat_map(read).collect()
}

fn world_at(level: Level) -> World {
    let mut world = platform();
    *world.level_mut() = level;
    world
}

#[test]
fn a_player_joins_at_the_worlds_time_and_difficulty() {
    let level = Level { spawn: None, time: 13_000, daylight_cycle: true };
    let mut h = Harness::with(ShardConfig { difficulty: Difficulty::Hard, ..config() }, world_at(level));
    h.join(1, "Steve");
    h.tick(1);
    h.receive(1, &pack_response(PackStatus::HaveAllPacks));
    h.tick(1);
    h.receive(1, &pack_response(PackStatus::Completed));
    let sent = h.tick(1);
    let start = &all::<StartGame>(&sent)[0];
    let cycle: Vec<_> = start.gamerules.iter().filter(|rule| rule.name.eq_ignore_ascii_case("dodaylightcycle")).map(|rule| rule.value.clone()).collect();
    assert_eq!((start.difficulty, cycle), (3, vec![GameRuleI32Value::Bool(true)]));
    // The join runs before the clock in a tick: two ticks had passed when this one told the time.
    assert_eq!(clocks(&sent), [(13_002, false)]);
}

#[test]
fn the_clock_runs_is_set_and_reminds_players() {
    let mut h = Harness::with(config(), world_at(Level { spawn: None, time: 100, daylight_cycle: true }));
    h.enter(1, "Steve");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let times = seen.clone();
    h.remote.run(move |ctx| {
        times.lock().unwrap().push(ctx.time());
        ctx.set_time(18_000);
        times.lock().unwrap().push(ctx.time());
    });
    let sent = h.tick(1);
    let before = seen.lock().unwrap().clone();
    assert!((101..130).contains(&before[0]) && before[1] == 18_000, "{before:?}");
    assert_eq!(clocks(&sent), [(18_000, false)]);

    // Within any 256 ticks a reminder goes out, with the time the ticks since added.
    let reminders: Vec<(usize, Vec<(i32, bool)>)> = (1..=256).map(|tick| (tick, clocks(&h.tick(1)))).filter(|(_, clocks)| !clocks.is_empty()).collect();
    let [(after, reminder)] = &reminders[..] else { panic!("{reminders:?}") };
    assert_eq!(reminder, &[(18_000 + *after as i32, false)]);
}

#[test]
fn a_stopped_daylight_cycle_keeps_the_time() {
    let mut h = Harness::with(config(), world_at(Level { spawn: None, time: 6_000, daylight_cycle: false }));
    h.enter(1, "Steve");
    let reminders: Vec<(i32, bool)> = (0..300).flat_map(|_| clocks(&h.tick(1))).collect();
    assert!(!reminders.is_empty() && reminders.iter().all(|reminder| *reminder == (6_000, true)), "{reminders:?}");
}

#[test]
fn the_spawn_can_be_moved() {
    let mut h = Harness::new();
    h.enter(1, "Steve");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let spawns = seen.clone();
    h.remote.run(move |ctx| {
        ctx.set_spawn([f32::NAN, 0.0, 0.0]);
        spawns.lock().unwrap().push(ctx.spawn());
        ctx.set_spawn([3.5, 70.0, -1.5]);
        spawns.lock().unwrap().push(ctx.spawn());
    });
    h.tick(1);
    assert_eq!(*seen.lock().unwrap(), [[0.5, 65.0, 0.5], [3.5, 70.0, -1.5]]);
}

fn actor(id: i64, identifier: &str, feet: [f32; 3], variant: i32) -> Vec<u8> {
    let floats = |items: &[f32]| Value::List(List { tag: 5, items: items.iter().map(|&item| Value::Float(item)).collect() });
    let fields = [
        ("UniqueID", Value::Long(id)),
        ("identifier", Value::String(identifier.into())),
        ("Pos", floats(&feet)),
        ("Rotation", floats(&[90.0, 0.0])),
        ("Variant", Value::Int(variant)),
    ];
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(fields.map(|(key, value)| (key.to_string(), value)).into()) });
    out.to_vec()
}

/// Joins `conn` and ticks until it is in the world; returns everything it was sent.
fn enter_stored(h: &mut Harness, conn: u64) -> Vec<RawPacket> {
    h.join_to_radius(conn, 2);
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut sent = Vec::new();
    while all::<PlayStatus>(&sent).is_empty() {
        assert!(Instant::now() < deadline, "the player never spawned");
        sent.extend(h.tick(conn));
        std::thread::sleep(Duration::from_millis(1));
    }
    h.receive(conn, &SetLocalPlayerAsInitialized { runtime_entity_id: 0 });
    (0..3).for_each(|_| sent.extend(h.tick(conn)));
    sent
}

#[test]
fn stored_mobs_are_shown_and_leave_with_their_column() {
    let storage = Arc::new(MemoryStorage::default());
    let chunk = Chunk::empty(0, 0, Dimension::overworld(BlockRegistry::vanilla().air_id()));
    let actors = [actor(-5, "minecraft:horse", [4.5, 64.0, 6.5], 3), actor(-6, "minecraft:item", [5.0, 64.0, 5.0], 0), actor(-7, "minecraft:bat", [9.5, 70.0, 2.5], 0)].concat();
    storage.store_column(ChunkPos::new(0, 0), 0, &StoredColumn { actors, ..StoredColumn::of(chunk) }).unwrap();

    let mut h = Harness::with(config(), World::with_storage(storage));
    let sent = enter_stored(&mut h, 1);
    let mut mobs = all::<AddEntity>(&sent);
    mobs.sort_by(|a, b| a.entity_type.cmp(&b.entity_type));
    let shown: Vec<_> = mobs.iter().map(|mob| (mob.entity_type.as_str(), [mob.position.x, mob.position.y, mob.position.z], mob.yaw)).collect();
    assert_eq!(shown, [("minecraft:bat", [9.5, 70.0, 2.5], 90.0), ("minecraft:horse", [4.5, 64.0, 6.5], 90.0)], "items are not shown yet");
    let ids: Vec<u64> = mobs.iter().map(|mob| mob.runtime_id).collect();
    assert!(ids[0] != ids[1] && mobs.iter().all(|mob| mob.unique_id == mob.runtime_id as i64));
    assert!((0..300).all(|_| all::<AddEntity>(&h.tick(1)).is_empty()), "a mob is announced once");

    // Nobody sees the column: it is unloaded and its mobs go; whoever comes next meets new ones.
    h.leave(1);
    (0..150).for_each(|_| h.shard.tick());
    let again = all::<AddEntity>(&enter_stored(&mut h, 2));
    assert_eq!(again.len(), 2);
    assert!(again.iter().all(|mob| !ids.contains(&mob.runtime_id)), "the reloaded column's mobs are new entities");
    assert!(all::<RemoveEntity>(&h.tick(2)).is_empty());
}
