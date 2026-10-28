//! What a world brings besides its blocks: the time of day, the difficulty, and its stored mobs.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Network, Value};
use acacia_proto::packets::{
    AddEntity, AddItemEntity, AddPainting, BlockEntityData, GameRulesChanged, InventoryContent, MobEquipment, RemoveEntity, ResourcePackClientResponseResponseStatus as PackStatus,
    SetDifficulty, SetSpawnPosition, SetSpawnPositionSpawnType as SpawnType, StartGame, Subchunk, SubchunkRequest, SyncWorldClocks,
    SyncWorldClocksContent as Clocks,
};
use acacia_proto::types::{GameRuleI32Value, MetadataDictionaryItemValue as Metadata, MetadataFlags1, Vec3i8, Vec3li};
use bytes::BytesMut;
use common::stored::{actor, compound, enter_stored, floor_holding};
use common::{Harness, all, click, config, pack_response, platform};
use sapling_api::{Difficulty, Item, PlayerId, RuleValue, Slot};
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
    let level = Level { time: 13_000, ..Level::default() };
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
    // BDS repeats both after StartGame, and says the player has no spawn of its own.
    assert_eq!(all::<SetDifficulty>(&sent)[0].difficulty, 3);
    assert_eq!(all::<GameRulesChanged>(&sent)[0].rules, start.gamerules);
    assert_eq!(all::<SetSpawnPosition>(&sent)[0].spawn_type, SpawnType::Player);
    // The join runs before the clock in a tick: two ticks had passed when this one told the time.
    assert_eq!(clocks(&sent), [(13_002, false)]);
}

#[test]
fn the_clock_runs_is_set_and_reminds_players() {
    let mut h = Harness::with(config(), world_at(Level { time: 100, ..Level::default() }));
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
    let mut h = Harness::with(config(), world_at(Level { time: 6_000, ..Level::default() }));
    h.enter(1, "Steve");
    h.remote.run(|ctx| {
        assert!(!ctx.set_game_rule("dodaylightcycle", RuleValue::Int(0)), "a switch takes no number");
        assert!(ctx.set_game_rule("dodaylightcycle", RuleValue::Bool(false)));
        assert_eq!(ctx.game_rule("DODAYLIGHTCYCLE"), Some(("doDayLightCycle".to_owned(), RuleValue::Bool(false))));
    });
    let sent = h.tick(1);
    let told: Vec<_> = all::<GameRulesChanged>(&sent).iter().flat_map(|changed| changed.rules.iter().map(|rule| (rule.name.clone(), rule.value.clone()))).collect();
    assert_eq!(told, [("doDayLightCycle".to_owned(), GameRuleI32Value::Bool(false))], "only the rule that changed, as BDS");
    let stopped_at = clocks(&sent);
    assert!(matches!(stopped_at[..], [(_, true)]), "{stopped_at:?}");
    let reminders: Vec<(i32, bool)> = (0..300).flat_map(|_| clocks(&h.tick(1))).collect();
    assert!(!reminders.is_empty() && reminders.iter().all(|reminder| *reminder == stopped_at[0]), "{reminders:?}");
    h.remote.run(|ctx| assert!(ctx.set_game_rule("doDayLightCycle", RuleValue::Bool(false))));
    assert!(all::<GameRulesChanged>(&h.tick(1)).is_empty(), "no change, nothing told");
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
    let told = all::<SetSpawnPosition>(&h.tick(1));
    assert_eq!(*seen.lock().unwrap(), [[0.5, 65.0, 0.5], [3.5, 70.0, -1.5]]);
    let blocks: Vec<_> = told.iter().map(|spawn| (spawn.spawn_type, [spawn.player_position.x, spawn.player_position.y, spawn.player_position.z])).collect();
    assert_eq!(blocks, [(SpawnType::World, [3, 70, -2])], "the players are told of the spawn that was taken");
}

#[test]
fn the_difficulty_can_be_changed() {
    let mut h = Harness::new();
    h.enter(1, "Steve");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let difficulties = seen.clone();
    h.remote.run(move |ctx| {
        difficulties.lock().unwrap().push(ctx.difficulty());
        ctx.set_difficulty(Difficulty::Normal);
        difficulties.lock().unwrap().push(ctx.difficulty());
    });
    let told: Vec<u32> = all::<SetDifficulty>(&h.tick(1)).iter().map(|told| told.difficulty).collect();
    assert_eq!((seen.lock().unwrap().clone(), told), (vec![Difficulty::Peaceful, Difficulty::Normal], vec![2]));
}

#[test]
fn a_stored_block_entity_comes_with_its_section_and_after_a_resync() {
    let registry = BlockRegistry::vanilla();
    let mut chunk = Chunk::empty(0, 0, Dimension::overworld(registry.air_id()));
    chunk.set(3, 67, 3, 0, registry.states_of("minecraft:chest").next().unwrap().0);
    let chest = compound([("id", Value::String("Chest".into())), ("x", Value::Int(3)), ("y", Value::Int(67)), ("z", Value::Int(3)), ("CustomName", Value::String("Loot".into()))]);
    let mut block_entities = BytesMut::new();
    nbt::write::<LittleEndian>(&mut block_entities, &Nbt { name: String::new(), value: chest.clone() });
    let storage = Arc::new(MemoryStorage::default());
    storage.store_column(ChunkPos::new(0, 0), 0, &StoredColumn { block_entities: block_entities.to_vec(), ..StoredColumn::of(chunk) }).unwrap();

    let mut h = Harness::with(config(), World::with_storage(storage));
    enter_stored(&mut h, 1);
    let mut wire = BytesMut::new();
    nbt::write::<Network>(&mut wire, &Nbt { name: String::new(), value: chest.clone() });
    h.receive(1, &SubchunkRequest { dimension: 0, requests: vec![Vec3i8 { x: 0, y: 4, z: 0 }], origin: Vec3li { x: 0, y: 0, z: 0 } });
    let payload = all::<Subchunk>(&h.tick(1))[0].entries[0].payload.clone().expect("a payload");
    assert!(payload.ends_with(&wire) && payload.len() > wire.len(), "the chest's data follows its section");

    // A refused click is answered with the blocks as they are, which would leave the chest nameless.
    struct Refuses;
    impl sapling_api::Handler for Refuses {
        fn on_interact(&mut self, _: &mut sapling_api::ShardCtx<'_>, _: &sapling_api::Player, event: &mut sapling_api::InteractEvent) {
            event.cancel();
        }
    }
    h.shard.add_handler(Box::new(Refuses));
    h.receive(1, &click([3, 67, 3], 1, None));
    let shown = all::<BlockEntityData>(&h.tick(1));
    assert_eq!(shown.iter().map(|data| (data.position.y, data.nbt.decode().unwrap().value)).collect::<Vec<_>>(), [(67, chest)]);
}

#[test]
fn a_players_items_wait_in_storage_for_the_next_visit() {
    let storage = Arc::new(MemoryStorage::default());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    let uuid = Arc::new(Mutex::new(String::new()));
    let steve = uuid.clone();
    h.remote.run(move |ctx| {
        ctx.set_item(PlayerId(1), Slot::Main(7), Some(&Item::new("minecraft:diamond", 5)));
        ctx.set_item(PlayerId(1), Slot::Offhand, Some(&Item::new("minecraft:shield", 1)));
        assert!(ctx.hurt_player(PlayerId(1), 6.0, sapling_api::DamageCause::Generic));
        assert!(ctx.give_effect(PlayerId(1), sapling_api::Effect::Speed, 6000, 1, true));
        *steve.lock().unwrap() = ctx.players().next().unwrap().uuid.to_string();
    });
    h.tick(1);
    h.leave(1);
    h.shard.tick();

    // Back at once: the save may still be on its way, and the items are there all the same.
    let mut back = enter_stored(&mut h, 2);
    (0..3).for_each(|_| back.extend(h.tick(2)));
    let effects: Vec<(i32, i32, i32)> =all::<acacia_proto::packets::MobEffect>(&back).iter().map(|told| (told.effect_id, told.amplifier, told.duration)).collect();
    assert!(matches!(effects[..], [(1, 1, 5800..=6000)]), "its speed waited, with the ticks it had left: {effects:?}");
    let found = Arc::new(Mutex::new(Vec::new()));
    let items = found.clone();
    h.remote.run(move |ctx| {
        assert_eq!(ctx.health(PlayerId(2)), Some(14.0), "its health waited too");
        items.lock().unwrap().extend([Slot::Main(7), Slot::Offhand, Slot::Main(0)].map(|slot| ctx.item(PlayerId(2), slot)));
        ctx.give(PlayerId(2), &Item::new("minecraft:diamond", 1));
    });
    let windows = all::<InventoryContent>(&h.tick(2));
    assert_eq!(*found.lock().unwrap(), [Some(Item::new("minecraft:diamond", 5)), Some(Item::new("minecraft:shield", 1)), None]);
    assert_eq!((windows.len(), windows[0].input[7].count, windows[3].input[0].count), (4, 6, 1));
    h.leave(2);
    h.shard.tick();
    h.shard.save_worlds();
    let kept = storage.load_player(&uuid.lock().unwrap()).unwrap().unwrap();
    assert_eq!((kept.vitals.map(|vitals| vitals[0]), kept.bed), (Some(14.0), None));
    assert_eq!(kept.effects.iter().map(|effect| (effect[0], effect[1], effect[3])).collect::<Vec<_>>(), [(1, 1, 1)]);
    assert_eq!((kept.main[7].as_ref().map(|stack| (stack.name.as_str(), stack.count)), kept.offhand.map(|stack| stack.name)), (Some(("minecraft:diamond", 6)), Some("minecraft:shield".into())));
}

#[test]
fn stored_mobs_are_shown_and_leave_with_their_column() {
    let stack = |name: &str| ("Item", compound([("Name", Value::String(name.into())), ("Count", Value::Byte(5)), ("Damage", Value::Short(0))]));
    let actors = [
        actor(-5, "minecraft:horse", [4.5, 64.0, 6.5], ("Saddled", Value::Byte(1))),
        actor(-9, "minecraft:painting", [6.5, 66.5, 1.03125], ("Motif", Value::String("Kebab".into()))),
        actor(-6, "minecraft:item", [5.0, 64.0, 5.0], stack("minecraft:diamond")),
        actor(-7, "minecraft:bat", [9.5, 70.0, 2.5], ("Mainhand", Value::List(List { tag: 10, items: vec![stack("minecraft:iron_sword").1] }))),
        actor(-8, "minecraft:item", [6.0, 64.0, 5.0], stack("sapling:no_such_item")),
    ]
    .concat();

    // The painting's wall, else it falls.
    let mut h = Harness::with(config(), World::with_storage(floor_holding(actors, &[([6, 66, 0], "minecraft:stone")])));
    let sent = enter_stored(&mut h, 1);
    let mut mobs = all::<AddEntity>(&sent);
    mobs.sort_by(|a, b| a.entity_type.cmp(&b.entity_type));
    let shown: Vec<_> = mobs.iter().map(|mob| (mob.entity_type.as_str(), mob.yaw)).collect();
    assert_eq!(shown, [("minecraft:bat", 90.0), ("minecraft:horse", 90.0)]);
    // The horse stands on the floor where it was kept; the bat has begun to fly about from there.
    let near = |mob: &AddEntity, [x, y, z]: [f32; 3]| (mob.position.x - x).abs() < 1.0 && (mob.position.y - y).abs() < 1.0 && (mob.position.z - z).abs() < 1.0;
    let at = |mob: &AddEntity| [mob.position.x, mob.position.y, mob.position.z];
    assert!(near(&mobs[0], [9.5, 70.0, 2.5]) && at(&mobs[1]) == [4.5, 64.0, 6.5], "{:?}", mobs.iter().map(at).collect::<Vec<_>>());
    assert!(mobs.iter().all(|mob| !mob.attributes.is_empty()), "each as BDS announces its kind");
    let saddled = |mob: &AddEntity| mob.metadata.iter().any(|item| matches!(&item.value, Metadata::Flags(flags) if flags.contains(MetadataFlags1::SADDLED)));
    assert_eq!(mobs.iter().map(saddled).collect::<Vec<_>>(), [false, true], "the horse wears what it was stored with");
    let held: Vec<_> = all::<MobEquipment>(&sent).iter().map(|held| (held.runtime_entity_id, held.item.count)).collect();
    assert_eq!(held, [(mobs[0].runtime_id, 5)], "the bat holds what it was stored with");
    assert_eq!(all::<AddPainting>(&sent).iter().map(|painting| painting.title.as_str()).collect::<Vec<_>>(), ["Kebab"]);
    let items = all::<AddItemEntity>(&sent);
    let lying: Vec<_> = items.iter().map(|item| (item.item.count, [item.position.x, item.position.y, item.position.z])).collect();
    // Shown at the middle of its box, which rests on the floor where it was stored.
    assert_eq!(lying, [(5, [5.0, 64.125, 5.0])], "the item no client knows is left out");
    let ids: Vec<u64> = mobs.iter().map(|mob| mob.runtime_id).chain([items[0].runtime_entity_id]).collect();
    assert!(ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2] && mobs.iter().all(|mob| mob.unique_id == mob.runtime_id as i64));
    assert!((0..300).all(|_| all::<AddEntity>(&h.tick(1)).is_empty()), "a mob is announced once");

    // Nobody sees the column: it is unloaded and its mobs go; whoever comes next meets new ones.
    h.leave(1);
    // Two sweeps: the mobs moved since the last, so the first saves the column and the second drops it.
    (0..250).for_each(|_| h.shard.tick());
    let sent = enter_stored(&mut h, 2);
    let again = all::<AddEntity>(&sent);
    assert_eq!((again.len(), all::<AddItemEntity>(&sent).len(), all::<AddPainting>(&sent).len()), (2, 1, 1));
    assert!(again.iter().all(|mob| !ids.contains(&mob.runtime_id)), "the reloaded column's mobs are new entities");
    assert!(all::<RemoveEntity>(&h.tick(2)).is_empty());
}
