//! What the generator leaves in a structure: the dungeon at 123,-20,270 of seed 20261003 (BDS put
//! a spider spawner and a chest there, docs/research/structure-block-entities.md).

use crate::common;

use std::sync::{Arc, OnceLock};

use acacia_proto::nbt::Value;
use acacia_proto::packets::{ContainerOpen, InventoryContent, PlayerAuthInput};
use acacia_proto::types::InputData;
use common::{Harness, all, click, input_at};
use sapling_api::{Difficulty, GameMode, PlayerId};
use sapling_world::{ChunkPos, Generated, MemoryStorage, StoredColumn, World, WorldStorage};

use crate::spawners::ask;

const ALEX: PlayerId = PlayerId(1);
const SEED: i64 = 20261003;
const SPAWNER: [i32; 3] = [123, -20, 270];
const CHEST: [i32; 3] = [125, -20, 270];
const FEET: [f32; 3] = [124.5, -20.0, 271.5];
const MIDNIGHT: i64 = 18000;

/// The dungeon's column as the generator makes it, generated once (seconds in a debug build).
fn dungeon_column() -> &'static StoredColumn {
    static COLUMN: OnceLock<StoredColumn> = OnceLock::new();
    COLUMN.get_or_init(|| Generated::new(MemoryStorage::default(), SEED).generate_column(ChunkPos::new(7, 16)))
}

/// Alex in the dungeon at midnight, its column the only one the world holds.
fn in_the_dungeon() -> Harness {
    let storage = MemoryStorage::default();
    storage.store_column(ChunkPos::new(7, 16), 0, dungeon_column()).unwrap();
    let mut h = Harness::with(common::config(), World::with_storage(Arc::new(storage)));
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_time(MIDNIGHT);
        ctx.teleport(ALEX, FEET);
    });
    for _ in 0..200 {
        if ask(&mut h, |ctx| ctx.block(SPAWNER)).is_some_and(|block| block.name.ends_with("mob_spawner")) {
            h.receive(1, &PlayerAuthInput { input_data: vec![InputData::HandledTeleport], ..input_at(FEET) });
            h.tick_all();
            return h;
        }
    }
    panic!("the dungeon never loaded");
}

fn entity_at(h: &mut Harness, pos: [i32; 3]) -> Value {
    ask(h, move |ctx| ctx.block_entity(pos).and_then(|data| data.get::<Value>().cloned())).expect("a block entity")
}

#[test]
fn a_generated_dungeons_spawner_spawns_its_spider() {
    let mut h = in_the_dungeon();
    assert_eq!(entity_at(&mut h, SPAWNER).get("EntityIdentifier"), Some(&Value::String("minecraft:spider".into())));
    // The column sweep starts it within 100 ticks; then its saved Delay of 20.
    let came = (0..150).any(|_| ask(&mut h, |ctx| ctx.entities().into_iter().any(|e| e.kind == "minecraft:spider")));
    assert!(came, "no spider came");
}

#[test]
fn a_generated_dungeons_chest_fills_when_first_opened() {
    let mut h = in_the_dungeon();
    let entity = entity_at(&mut h, CHEST);
    assert_eq!(entity.get("LootTable"), Some(&Value::String("loot_tables/chests/simple_dungeon.json".into())));
    assert_eq!(entity.get("LootTableSeed"), Some(&Value::Int(357601146)), "BDS's seed");
    h.receive(1, &click(CHEST, 1, None));
    let sent = h.tick_all().to(1).to_vec();
    let window = all::<ContainerOpen>(&sent).first().expect("the chest opens").window_id.to_raw();
    let shown = all::<InventoryContent>(&sent).into_iter().find(|content| content.window_id.to_raw() == window).expect("its slots");
    assert!(shown.input.iter().any(|item| item.count > 0), "the chest is empty");
    let entity = entity_at(&mut h, CHEST);
    assert_eq!(entity.get("LootTable"), None, "rolled once");
    assert!(matches!(entity.get("Items"), Some(Value::List(items)) if !items.items.is_empty()));
}
