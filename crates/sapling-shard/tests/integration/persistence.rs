//! A mob's `Persistent` and `NaturalSpawn` through a world folder (docs/research/mob-spawning.md
//! "Despawning"): natural mobs saved so come back despawning, summoned ones stay.

use crate::common;

use std::sync::Arc;

use acacia_proto::nbt::Value;
use common::stored::{actor_with, enter_stored, floor_with};
use common::Harness;
use sapling_api::{Difficulty, GameMode, PlayerId};
use sapling_shard::ShardConfig;
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, MemoryStorage, StoredColumn, World, WorldStorage};

const ALEX: PlayerId = PlayerId(1);
const ZOMBIE: &str = "minecraft:zombie";
/// No natural rule spawns it over a plains floor.
const SUMMONED: &str = "minecraft:husk";
const BRUTE: &str = "minecraft:piglin_brute";

fn normal() -> ShardConfig {
    ShardConfig { difficulty: Difficulty::Normal, ..common::config() }
}

/// The ids of the mobs of `kind`: made from storage in its order, so oldest first.
fn of_kind(h: &mut Harness, kind: &'static str) -> Vec<u64> {
    let found = Arc::new(std::sync::Mutex::new(Vec::new()));
    let out = found.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.entities().into_iter().filter(|entity| entity.kind == kind).map(|entity| entity.id.0).collect());
    h.tick_all();
    let mut found = found.lock().unwrap().clone();
    found.sort_unstable();
    found
}

#[test]
fn a_mob_saved_as_natural_comes_back_despawning_and_a_persistent_one_stays() {
    let flags = |persistent: i8, natural: i8| vec![("Persistent", Value::Byte(persistent)), ("NaturalSpawn", Value::Byte(natural))];
    let mobs = [(-11, ZOMBIE, 12.5, flags(0, 1)), (-12, ZOMBIE, 13.5, flags(1, 0)), (-13, ZOMBIE, 14.5, flags(0, 0)), (-14, BRUTE, 12.5, flags(0, 1))];
    let actors = mobs.into_iter().map(|(id, kind, x, flags)| actor_with(id, kind, [x, 64.0, (-id - 10) as f32 * 3.0], flags)).collect::<Vec<_>>().concat();
    let mut h = Harness::with(normal(), World::with_storage(floor_with(actors)));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    let loaded = of_kind(&mut h, ZOMBIE);
    assert_eq!((loaded.len(), of_kind(&mut h, BRUTE).len()), (3, 1));
    // 46 off: past BDS's far distance of 44 at tick distance 4, its column still in view.
    h.remote.run(|ctx| ctx.teleport(ALEX, [-33.5, 64.0, 4.5]));
    (0..3).for_each(|_| drop(h.tick_all()));
    assert_eq!(of_kind(&mut h, ZOMBIE), loaded[1..], "the natural one went; one saved with no NaturalSpawn stays as before");
    assert_eq!(of_kind(&mut h, BRUTE).len(), 1, "a brute's filters go from 54");
    h.remote.run(|ctx| ctx.teleport(ALEX, [-45.5, 64.0, 4.5]));
    (0..3).for_each(|_| drop(h.tick_all()));
    assert!(of_kind(&mut h, BRUTE).is_empty());
}

/// Stone at y 63 under the chunks -3 to 2 each way.
fn stored_floor() -> Arc<MemoryStorage> {
    let registry = BlockRegistry::vanilla();
    let stone = registry.states_of("minecraft:stone").next().unwrap().0;
    let storage = Arc::new(MemoryStorage::default());
    for (cx, cz) in (-3..3).flat_map(|cx| (-3..3).map(move |cz| (cx, cz))) {
        let mut chunk = Chunk::empty(cx, cz, Dimension::overworld(registry.air_id()));
        (0..16).for_each(|x| (0..16).for_each(|z| chunk.set(x, 63, z, 0, stone)));
        storage.store_column(ChunkPos::new(cx, cz), 0, &StoredColumn::of(chunk)).unwrap();
    }
    storage
}

#[test]
fn natural_mobs_are_saved_not_persistent_and_summoned_ones_persistent() {
    let storage = stored_floor();
    let mut h = Harness::with(normal(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_time(18000);
        assert!(ctx.spawn_entity(SUMMONED, [2.5, 64.0, 2.5]).is_some());
    });
    (0..400).for_each(|_| drop(h.tick_all()));
    h.shard.save_worlds();
    let mut seen = Vec::new();
    for (cx, cz) in (-3..3).flat_map(|cx| (-3..3).map(move |cz| (cx, cz))) {
        let Some(column) = storage.load_column(ChunkPos::new(cx, cz), 0).unwrap() else { continue };
        for (actor, _, compound) in sapling_world::stored_actors(&column.actors).unwrap() {
            let Some(actor) = actor.filter(|actor| actor.identifier != "minecraft:item") else { continue };
            let kept = sapling_world::raising(&compound).unwrap();
            seen.push((actor.identifier == SUMMONED, kept.persistent, kept.natural));
        }
    }
    assert!(seen.contains(&(true, true, false)), "the summoned husk: {seen:?}");
    assert!(seen.iter().filter(|(summoned, ..)| !summoned).all(|&(_, persistent, natural)| !persistent && natural), "{seen:?}");
    assert!(seen.len() > 1, "a natural monster came: {seen:?}");
}
