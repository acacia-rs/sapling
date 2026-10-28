//! Maps kept with the world across a restart (maps/kept.rs; BDS's `map_<id>` records).

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::{ClientboundMapItemData, MapInfoRequest};
use common::shooting::click_air;
use common::stored::{enter_stored, floor_holding, plain};
use common::{Harness, all, config};
use sapling_world::{ChunkPos, Level, MemoryStorage, PlayerData, StorageError, StoredColumn, World, WorldStorage};

/// Storage in memory that remembers the level it was last flushed with.
struct Kept {
    columns: Arc<MemoryStorage>,
    level: Mutex<Option<Level>>,
}

impl WorldStorage for Kept {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        self.columns.load_column(pos, dimension)
    }
    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError> {
        self.columns.store_column(pos, dimension, column)
    }
    fn load_player(&self, id: &str) -> Result<Option<PlayerData>, StorageError> {
        self.columns.load_player(id)
    }
    fn store_player(&self, id: &str, player: &PlayerData) -> Result<(), StorageError> {
        self.columns.store_player(id, player)
    }
    fn flush(&self, level: &Level) -> Result<(), StorageError> {
        *self.level.lock().unwrap() = Some(level.clone());
        Ok(())
    }
}

fn opened(storage: &Arc<Kept>) -> Harness {
    let mut world = World::with_storage(storage.clone());
    if let Some(level) = storage.level.lock().unwrap().clone() {
        *world.level_mut() = level;
    }
    Harness::with(config(), world)
}

#[test]
fn a_drawn_map_is_kept_and_shown_again_after_a_restart() {
    let columns = floor_holding(Vec::new(), &[]);
    columns.store_player(&acacia_auth::login::offline_identity("Steve"), &PlayerData { main: vec![plain("minecraft:empty_map")], ..PlayerData::default() }).unwrap();
    let storage = Arc::new(Kept { columns, level: Mutex::new(None) });
    let mut h = opened(&storage);
    enter_stored(&mut h, 1);
    h.receive(1, &click_air());
    let mut sent = Vec::new();
    (0..30).for_each(|_| sent.extend(h.tick(1)));
    let id = all::<ClientboundMapItemData>(&sent).first().expect("the map made").map_id;
    h.leave(1);
    h.shard.tick();
    h.shard.save_worlds();
    let level = storage.level.lock().unwrap().clone().expect("flushed");
    let [record] = &level.maps[..] else { panic!("{} maps kept", level.maps.len()) };
    assert_eq!((record.id, record.parent, record.scale, record.centre, record.locked), (id, id + 1, 0, [0, 0], false));
    assert!(record.colours.iter().any(|byte| *byte != 0), "the drawing is kept");
    drop(h);

    let mut h = opened(&storage);
    enter_stored(&mut h, 1);
    h.receive(1, &MapInfoRequest { map_id: id, client_pixels: Vec::new() });
    let shown = all::<ClientboundMapItemData>(&h.tick(1)).into_iter().find(|packet| packet.pixels.is_some()).expect("the kept map");
    assert_eq!((shown.map_id, shown.scale, shown.width), (id, Some(0), Some(128)));
    let kept: Vec<u32> = record.colours.chunks_exact(4).map(|rgba| u32::from_le_bytes(rgba.try_into().unwrap())).collect();
    let pixels: Vec<u32> = shown.pixels.unwrap().iter().map(|pixel| *pixel as u32).collect();
    assert!(pixels == kept, "the kept drawing");
}
