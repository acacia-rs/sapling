//! Natural spawning and despawning (docs/research/mob-spawning.md) and spawn eggs.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::nbt::Value;
use common::raising::use_on;
use common::stored::{compound, enter_stored};
use common::{Harness, click};
use sapling_api::{Difficulty, EntityInfo, GameMode, Item, PlayerId, RuleValue, ShardCtx, Slot};
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, ItemStack, MemoryStorage, PlayerData, SpawnArea, StoredColumn, World, WorldStorage, encode_spawn_areas};

const ALEX: PlayerId = PlayerId(1);
const MIDNIGHT: i64 = 18000;
/// 30 off chunk (0, 0)'s middle: structure spawns want a player 24 to 44 away.
const FROM_THE_MIDDLE: [f32; 3] = [8.5, 64.0, 38.5];
const MONSTERS: &[&str] = &["minecraft:zombie", "minecraft:zombie_villager_v2", "minecraft:skeleton", "minecraft:spider", "minecraft:creeper", "minecraft:enderman", "minecraft:witch"];

/// A void world with a floor of `floor` at y 64 over the four chunks each way of the origin: every
/// spawn's cap counts all of it.
fn floored(floor: &str) -> World {
    let mut level = World::void();
    let id = level.block_id(floor).unwrap();
    level.fill([-32, 64, -32], [31, 64, 31], id);
    level
}

/// Alex in creative, so nothing hunts it, at midnight on normal difficulty over `floor`.
fn at_midnight(floor: &str) -> Harness {
    at_midnight_with(common::config(), floor)
}

fn at_midnight_with(config: sapling_shard::ShardConfig, floor: &str) -> Harness {
    let mut h = Harness::with(config, floored(floor));
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_time(MIDNIGHT);
    });
    h.tick_all();
    h
}

fn ask<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> T {
    let found = Arc::new(Mutex::new(None));
    let out = found.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = Some(ask(ctx)));
    h.tick_all();
    found.lock().unwrap().take().expect("the handler ran")
}

fn monsters(h: &mut Harness) -> Vec<EntityInfo> {
    ask(h, |ctx| ctx.entities().into_iter().filter(|entity| MONSTERS.contains(&entity.kind.as_str())).collect())
}

fn run(h: &mut Harness, ticks: usize) {
    (0..ticks).for_each(|_| drop(h.tick_all()));
}

#[test]
fn monsters_appear_in_the_dark_away_from_the_player_up_to_their_cap() {
    let mut h = at_midnight("minecraft:stone");
    run(&mut h, 400);
    let found = monsters(&mut h);
    assert!(!found.is_empty(), "nothing spawned on a dark floor");
    for monster in &found {
        let across = (monster.feet[0] - 0.5).hypot(monster.feet[2] - 0.5);
        assert!((23.0..=45.0).contains(&across) && monster.feet[1] == 65.0, "{monster:?}");
    }
    run(&mut h, 1200);
    // The cap is checked before a herd of up to 4 comes.
    assert!(monsters(&mut h).len() <= 8 + 3, "{:?}", monsters(&mut h));
}

#[test]
fn a_lit_floor_spawns_nothing() {
    let mut h = at_midnight("minecraft:glowstone");
    run(&mut h, 200);
    assert_eq!(monsters(&mut h), []);
}

/// Block light 2 passes the pack rules' brightness 0 to 7, not Monster's own check (mob-spawning.md "Own spawn checks").
#[test]
fn a_dimly_lit_floor_spawns_no_monster() {
    let mut h = at_midnight("minecraft:magma");
    run(&mut h, 400);
    assert_eq!(monsters(&mut h), []);
}

#[test]
fn peaceful_and_do_mob_spawning_off_spawn_nothing() {
    let mut h = at_midnight("minecraft:stone");
    h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Peaceful));
    run(&mut h, 200);
    assert_eq!(monsters(&mut h), []);
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        assert!(ctx.set_game_rule("doMobSpawning", RuleValue::Bool(false)));
    });
    run(&mut h, 200);
    assert_eq!(monsters(&mut h), []);
}

/// `/away`: the sender 120 east, moved by a command as the drill's `/tp` moved it.
#[derive(sapling_api::Command)] #[command(crate = "sapling_api")]
struct Away;
impl sapling_api::Command for Away {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &sapling_api::Sender, _: &mut sapling_api::Output) { ctx.teleport(sender.player().expect("a player").id, [120.5, 65.0, 0.5]) }
}

/// bds-w3despawn.txt: mobs out of every tick region stay however long (`DespawnSystem` views only
/// `ActorTickedComponent`); back near them, the natural ones past 44 go. 12 chunks keep their columns
/// loaded 120 east; on the command's tick Alex's view still covers them.
#[test]
fn natural_mobs_out_of_every_tick_region_stay_and_go_once_ticked_again() {
    let mut h = at_midnight_with(sapling_shard::ShardConfig { max_view_radius: 12, ..common::config() }, "minecraft:stone");
    h.shard.add_command::<Away>();
    h.receive(1, &acacia_proto::packets::RequestChunkRadius { chunk_radius: 12, max_radius: 12 });
    run(&mut h, 400);
    let ids = |h: &mut Harness| monsters(h).into_iter().map(|monster| monster.id).collect::<Vec<_>>();
    let natural = ids(&mut h);
    let summoned = ask(&mut h, |ctx| ctx.spawn_entity("minecraft:zombie", [10.5, 65.0, 10.5])).expect("a zombie");
    let origin = acacia_proto::types::CommandOrigin { r#type: "player".into(), uuid: acacia_proto::manual::Uuid([1; 16]), request_id: "req".into(), player_entity_id: 0 };
    h.receive(1, &acacia_proto::packets::CommandRequest { command: "/away".into(), origin, internal: false, version: "latest".into() });
    run(&mut h, 1200);
    assert_eq!(ask(&mut h, |ctx| ctx.position(ALEX).map(|at| at.feet[0])), Some(120.5), "moved by the command");
    let left = ids(&mut h);
    assert!(natural.iter().chain([&summoned]).all(|id| left.contains(id)), "{natural:?} and {summoned:?}, then {left:?}");
    h.remote.run(|ctx| ctx.teleport(ALEX, [0.5, 135.0, 0.5]));
    run(&mut h, 3);
    let left = ids(&mut h);
    assert!(left.contains(&summoned) && !natural.is_empty() && natural.iter().all(|id| !left.contains(id)), "{natural:?}, then {left:?}");
}

/// BDS caps the far distance at 44 only where the chunk tick range is 4 or less (`_canDespawn`,
/// docs/research/mob-spawning.md "Despawning"): 70 blocks over them, its monsters go at 4 and stay at 6.
#[test]
fn natural_mobs_70_off_go_at_tick_distance_4_and_stay_at_6() {
    for (tick_distance, stay) in [(4, false), (6, true)] {
        let mut h = at_midnight_with(sapling_shard::ShardConfig { tick_distance, ..common::config() }, "minecraft:stone");
        run(&mut h, 400);
        let before: Vec<_> = monsters(&mut h).into_iter().filter(|monster| monster.feet[0].hypot(monster.feet[2]) < 46.0).map(|monster| monster.id).collect();
        assert!(!before.is_empty());
        h.remote.run(|ctx| ctx.teleport(ALEX, [0.5, 135.0, 0.5]));
        run(&mut h, 3);
        let after: Vec<_> = monsters(&mut h).into_iter().map(|monster| monster.id).collect();
        assert_eq!(before.iter().all(|id| after.contains(id)), stay, "tick distance {tick_distance}: {before:?} then {after:?}");
        assert!(stay || before.iter().all(|id| !after.contains(id)), "tick distance {tick_distance}");
    }
}

/// Stored stone at y 63 under the chunks -3 to 2 each way, and Steve holding a name tag for "Dolly".
fn stored_floor_with_a_name_tag() -> Arc<MemoryStorage> {
    let registry = BlockRegistry::vanilla();
    let stone = registry.states_of("minecraft:stone").next().unwrap().0;
    let storage = Arc::new(MemoryStorage::default());
    for (cx, cz) in (-3..3).flat_map(|cx| (-3..3).map(move |cz| (cx, cz))) {
        let mut chunk = Chunk::empty(cx, cz, Dimension::overworld(registry.air_id()));
        (0..16).for_each(|x| (0..16).for_each(|z| chunk.set(x, 63, z, 0, stone)));
        storage.store_column(ChunkPos::new(cx, cz), 0, &StoredColumn::of(chunk)).unwrap();
    }
    let display = compound([("Name", Value::String("Dolly".into()))]);
    let tag = ItemStack { name: "minecraft:name_tag".into(), count: 1, damage: 0, block: None, tag: Some(compound([("display", display)])) };
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &PlayerData { main: vec![Some(tag)], ..PlayerData::default() }).unwrap();
    storage
}

#[test]
fn a_natural_mob_a_player_names_stays_when_the_player_goes() {
    let mut h = Harness::with(common::config(), World::with_storage(stored_floor_with_a_name_tag()));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_time(MIDNIGHT);
    });
    run(&mut h, 400);
    let named = monsters(&mut h).into_iter().next().expect("a natural monster");
    let [x, y, z] = named.feet;
    h.remote.run(move |ctx| ctx.teleport(ALEX, [x + 1.0, y, z]));
    run(&mut h, 2);
    h.receive(1, &use_on(named.id.0));
    run(&mut h, 2);
    // 70 over them, where they are still ticked: a column out of every tick region keeps its natural mobs too (bds-w3despawn.txt).
    h.remote.run(|ctx| ctx.teleport(ALEX, [0.5, 135.0, 0.5]));
    run(&mut h, 3);
    let left: Vec<_> = monsters(&mut h).into_iter().map(|monster| monster.id).collect();
    assert_eq!(left, [named.id]);
}

/// Stored water at y 63 under the chunks -3 to 2 each way, where no land monster stands (glowstone
/// would light the piece past Monster's block light 0), but stone under chunk (0, 0), a piece of
/// `structure` whole (docs/research/structure-spawns.md).
fn stored_piece(structure: &str) -> Arc<MemoryStorage> {
    let registry = BlockRegistry::vanilla();
    let floor = |name| registry.states_of(name).next().unwrap().0;
    let storage = Arc::new(MemoryStorage::default());
    let area = |bounds, whole| SpawnArea { structure: structure.into(), bounds, whole, y_offset: Some(-3) };
    for (cx, cz) in (-3..3).flat_map(|cx| (-3..3).map(move |cz| (cx, cz))) {
        let mut chunk = Chunk::empty(cx, cz, Dimension::overworld(registry.air_id()));
        let block = floor(if (cx, cz) == (0, 0) { "minecraft:stone" } else { "minecraft:water" });
        (0..16).for_each(|x| (0..16).for_each(|z| chunk.set(x, 63, z, 0, block)));
        let mut column = StoredColumn::of(chunk);
        if (cx, cz) == (0, 0) {
            column.spawn_areas = encode_spawn_areas(&[area([0, 63, 0, 15, 69, 15], true), area([0, 63, 0, 15, 69, 15], false)]);
        }
        storage.store_column(ChunkPos::new(cx, cz), 0, &column).unwrap();
    }
    storage
}

#[test]
fn witches_appear_in_a_hut_piece_where_nothing_else_does() {
    let mut h = Harness::with(common::config(), World::with_storage(stored_piece("minecraft:swamp_hut")));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_time(MIDNIGHT);
    });
    run(&mut h, 1500);
    assert_eq!(monsters(&mut h), [], "peaceful");
    h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Normal));
    run(&mut h, 1500);
    assert_eq!(monsters(&mut h), [], "a player under 24 off");
    h.remote.run(|ctx| ctx.teleport(ALEX, FROM_THE_MIDDLE));
    run(&mut h, 3000);
    let found = monsters(&mut h);
    assert!(!found.is_empty(), "no witch in the hut");
    for monster in &found {
        let [x, y, z] = monster.feet;
        // At the middle of the piece, on the block's corner (1415efb20).
        assert!(monster.kind == "minecraft:witch" && [x, y, z] == [8.0, 64.0, 8.0], "{monster:?}");
    }
}

#[test]
fn an_outpost_keeps_to_its_pillager_caps() {
    let mut h = Harness::with(common::config(), World::with_storage(stored_piece("minecraft:pillager_outpost")));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_time(MIDNIGHT);
        ctx.teleport(ALEX, FROM_THE_MIDDLE);
    });
    run(&mut h, 6000);
    let pillagers = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:pillager").count());
    // The pillager's own cap of 7, and one captain while none stood there.
    assert!((1..=8).contains(&pillagers), "{pillagers} pillagers");
}

/// Water from y 40 to 62 over stone under the chunks -3 to 2 each way, a monument building over
/// chunk (0, 0) from 39 to 61.
fn stored_monument() -> Arc<MemoryStorage> {
    let registry = BlockRegistry::vanilla();
    let [stone, water] = ["minecraft:stone", "minecraft:water"].map(|name| registry.states_of(name).next().unwrap().0);
    let storage = Arc::new(MemoryStorage::default());
    let area = |whole| SpawnArea { structure: "minecraft:monument".into(), bounds: [0, 39, 0, 15, 61, 15], whole, y_offset: Some(0) };
    for (cx, cz) in (-3..3).flat_map(|cx| (-3..3).map(move |cz| (cx, cz))) {
        let mut chunk = Chunk::empty(cx, cz, Dimension::overworld(registry.air_id()));
        (0..16).for_each(|x| (0..16).for_each(|z| (39..=62).for_each(|y| chunk.set(x, y, z, 0, if y == 39 { stone } else { water }))));
        let mut column = StoredColumn::of(chunk);
        if (cx, cz) == (0, 0) {
            column.spawn_areas = encode_spawn_areas(&[area(true), area(false)]);
        }
        storage.store_column(ChunkPos::new(cx, cz), 0, &column).unwrap();
    }
    storage
}

#[test]
fn guardians_appear_and_stay_in_a_monument() {
    let config = sapling_shard::ShardConfig { mobs_stroll: true, ..common::config() };
    let mut h = Harness::with(config, World::with_storage(stored_monument()));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_time(6000);
        ctx.teleport(ALEX, FROM_THE_MIDDLE);
    });
    run(&mut h, 2000);
    let guardians = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:guardian").count());
    assert!(guardians > 0, "no guardian stayed");
}

#[test]
fn a_spawn_egg_hatches_on_the_face_clicked_and_is_spent_in_survival() {
    let mut h = Harness::with(common::config(), floored("minecraft:stone"));
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:pig_spawn_egg", 2)))));
    h.tick_all();
    h.receive(1, &click([2, 64, 0], 1, None));
    h.tick_all();
    let pigs = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:pig").map(|pig| pig.feet).collect::<Vec<_>>());
    assert_eq!(pigs, [[2.5, 65.0, 0.5]]);
    assert_eq!(ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|held| held.count)), Some(1));
}
