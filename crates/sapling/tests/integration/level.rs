//! A world folder's time, difficulty and mobs, end to end: a bot joins a server opened on a copy
//! of the sapling-leveldb fixture with a horse stored in it, and the folder is read back after
//! the server shut down.

use crate::common;

use std::path::PathBuf;
use std::time::Duration;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::nbt::{self, List, LittleEndian, Nbt, Value};
use acacia_bot::proto::packets::{AddEntity, StartGame, SyncWorldClocks, SyncWorldClocksContent as Clocks};
use acacia_bot::{BotConfig, BotEvent};
use bytes::BytesMut;
use common::{GROUND, SPAWN, fixture};
use sapling::{Handler, RuleValue, ShardCtx, World};
use sapling_leveldb::{ChunkKey, LevelDbWorld, actors};

const HORSE: [f32; 3] = [SPAWN.0 as f32 + 2.5, GROUND as f32 + 1.0, SPAWN.1 as f32 + 0.5];

fn horse() -> Vec<u8> {
    let floats = |items: &[f32]| Value::List(List { tag: 5, items: items.iter().map(|&item| Value::Float(item)).collect() });
    let fields = [
        ("UniqueID", Value::Long(-4_294_967_295)),
        ("identifier", Value::String("minecraft:horse".into())),
        ("Pos", floats(&HORSE)),
        ("Rotation", floats(&[45.0, 0.0])),
        ("Variant", Value::Int(3)),
    ];
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(fields.map(|(key, value)| (nbt::Str::from(key), value)).into()) });
    out.to_vec()
}

/// A copy of the fixture with a horse near the spawn; the caller removes it.
fn folder_with_a_horse() -> PathBuf {
    let copy = std::env::temp_dir().join(format!("sapling-level-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&copy);
    std::fs::create_dir_all(copy.join("db")).unwrap();
    for entry in std::fs::read_dir(fixture().join("db")).unwrap() {
        let file = entry.unwrap().file_name();
        std::fs::copy(fixture().join("db").join(&file), copy.join("db").join(&file)).unwrap();
    }
    std::fs::copy(fixture().join("level.dat"), copy.join("level.dat")).unwrap();
    let world = LevelDbWorld::open(&copy).unwrap();
    let key = spawn_column();
    let column = world.load_column(key).unwrap().unwrap();
    world.store_column(key, &column.chunk, &column.block_entities, &horse()).unwrap();
    world.flush(world.level()).unwrap();
    copy
}

fn spawn_column() -> ChunkKey {
    ChunkKey::new(SPAWN.0 >> 4, SPAWN.1 >> 4, 0)
}

/// Sets the time once somebody is there to be told, and two game rules.
struct Dusk;

impl Handler for Dusk {
    fn on_tick(&mut self, ctx: &mut ShardCtx<'_>) {
        if ctx.player_count() > 0 && ctx.time() < 100_000 {
            ctx.set_time(112_000);
            assert!(ctx.set_game_rule("randomTickSpeed", RuleValue::Int(3)) && ctx.set_game_rule("domobloot", RuleValue::Bool(false)));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_folder_brings_its_time_and_mobs_and_takes_the_time_back() {
    let folder = folder_with_a_horse();
    let opened = World::open_folder(&folder).expect("the copy opens");
    let saved_time = opened.world.level().time;
    let server = crate::common::builder().bind("127.0.0.1:0".parse().unwrap()).online_mode(false).folder(opened).handler(Dusk).start().await.expect("the server starts");

    let subscribe = PacketFilter::none().with(AddEntity::ID).with(SyncWorldClocks::ID).with(StartGame::ID);
    let mut bot = common::spawn(common::client(&server, "Rider"), BotConfig { subscribe, ..BotConfig::default() }).await;
    let (mut mobs, mut times, mut difficulty) = (Vec::new(), Vec::new(), None);
    let watch = tokio::time::timeout(Duration::from_secs(4), async {
        while let Some(event) = bot.next().await {
            let BotEvent::Packet(raw) = event else { continue };
            if raw.id == AddEntity::ID {
                mobs.push(raw.decode::<AddEntity>().expect("the bot decodes the mob"));
            } else if raw.id == StartGame::ID {
                difficulty = Some(raw.decode::<StartGame>().expect("StartGame decodes").difficulty);
            } else {
                match raw.decode::<SyncWorldClocks>().expect("the clock decodes").content {
                    Clocks::InitializeRegistry(registry) => times.push(("registry", registry.clocks[0].time)),
                    Clocks::SyncState(state) => times.push(("state", state.sync_states[0].time)),
                    other => panic!("{other:?}"),
                }
            }
        }
    });
    assert!(watch.await.is_err(), "the bot stayed connected");

    let shown: Vec<_> = mobs.iter().map(|mob| (mob.entity_type.as_str(), [mob.position.x, mob.position.y, mob.position.z], mob.yaw)).collect();
    assert_eq!(shown, [("minecraft:horse", HORSE, 45.0)]);
    assert_eq!(difficulty, Some(0), "the fixture is peaceful");
    let [("registry", joined), ("state", set), ..] = times[..] else { panic!("{times:?}") };
    assert!((saved_time..saved_time + 2_000).contains(&i64::from(joined)), "joined at {joined}, saved at {saved_time}");
    assert_eq!(set, 112_000);
    bot.disconnect().await;
    server.shutdown().await;

    let after = LevelDbWorld::open_in_memory(&folder).unwrap();
    let time = after.level().time().unwrap();
    assert!((112_000..114_000).contains(&time), "saved at {time}");
    let rules = (after.level().int("randomtickspeed"), after.level().root.get("domobloot"));
    assert_eq!(rules, (Some(3), Some(&Value::Byte(0))), "the rules are saved as BDS saves them");
    let stored = actors(&after.load_column(spawn_column()).unwrap().unwrap().actors).unwrap();
    assert_eq!(stored.iter().map(|actor| (actor.identifier.as_str(), actor.position, actor.variant)).collect::<Vec<_>>(), [("minecraft:horse", HORSE, 3)]);
    drop(after);
    std::fs::remove_dir_all(&folder).unwrap();
}
