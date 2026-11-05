use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use sapling_leveldb::{ChunkKey, LevelDat, LevelDbWorld, PlayerData};

use super::{StorageError, StoredColumn, WorldStorage};
use crate::ChunkPos;
use crate::level::{Level, read_rules, read_structures, write_rules};

/// A vanilla world folder.
pub struct LevelDbStorage(LevelDbWorld);

impl LevelDbStorage {
    /// Opens the folder's database in place (`sapling_leveldb::Db::open`).
    pub fn open(folder: &Path) -> Result<Self, sapling_leveldb::Error> {
        LevelDbWorld::open(folder).map(Self)
    }

    /// Serves the world from a copy in memory; the folder is only read.
    pub fn open_in_memory(folder: &Path) -> Result<Self, sapling_leveldb::Error> {
        LevelDbWorld::open_in_memory(folder).map(Self)
    }

    /// `level.dat` as it was when the folder was opened.
    pub fn level_dat(&self) -> &LevelDat {
        self.0.level()
    }

    /// What `level.dat` says of [`Level`]'s fields.
    pub fn level(&self) -> Level {
        let dat = self.0.level();
        let portals = self.0.load_portals().unwrap_or_else(|error| {
            tracing::warn!(%error, "the portal records are unreadable; portals find none");
            Vec::new()
        });
        let dragon_fight = self.0.load_dragon_fight().unwrap_or_else(|error| {
            tracing::warn!(%error, "the dragon fight is unreadable; a new one begins");
            None
        });
        let scoreboard = self.0.load_scoreboard().unwrap_or_else(|error| {
            tracing::warn!(%error, "the scoreboard is unreadable; it starts empty");
            None
        });
        let [folder_structures, structures] = read_structures(&self.0);
        let ticking_areas = self.0.load_ticking_areas().unwrap_or_else(|error| {
            tracing::warn!(%error, "the ticking areas are unreadable; there are none");
            Vec::new()
        });
        let maps = self.0.load_maps().unwrap_or_else(|error| {
            tracing::warn!(%error, "the maps are unreadable; there are none");
            Vec::new()
        });
        let (spawn, rules, seed, hardcore) = (dat.spawn(), read_rules(dat), dat.seed().unwrap_or(0), dat.hardcore());
        let (structures, folder_structures) = (structures.into(), folder_structures.into());
        let (time, weather, difficulty) = (dat.time().unwrap_or(Level::default().time), dat.weather().unwrap_or(Level::default().weather), dat.difficulty());
        let maps = maps.into();
        let (trackers, last_tracker) = self.0.load_trackers().unwrap_or_else(|error| {
            tracing::warn!(%error, "the lodestone targets are unreadable; compasses find none");
            (Vec::new(), 0)
        });
        let villages = self.0.load_villages().unwrap_or_else(|error| {
            tracing::warn!(%error, "the villages are unreadable; no raid goes on, no standing is kept");
            Vec::new()
        });
        let age = dat.age().unwrap_or(0);
        let mob_events = sapling_leveldb::load_mob_events(self.0.db()).unwrap_or_else(|error| {
            tracing::warn!(%error, "the mob event switches are unreadable; every one is on");
            None
        });
        Level { spawn, time, age, rules, portals, seed, dragon_fight, scoreboard, structures, folder_structures, ticking_areas, maps, trackers, last_tracker, villages, hardcore, weather, difficulty, chunk_tick_range: None, mob_events }
    }
}

/// The game version sapling writes worlds for, as `lastOpenedWithVersion` spells it.
fn game_version() -> [i32; 5] {
    let mut parts = acacia_proto::GAME_VERSION.split('.').map(|part| part.parse().unwrap_or(0));
    std::array::from_fn(|_| parts.next().unwrap_or(0))
}

impl WorldStorage for LevelDbStorage {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        let column = self.0.load_column(ChunkKey::new(pos.x, pos.z, dimension))?;
        Ok(column.map(|c| StoredColumn { block_entities: c.block_entities, actors: c.actors, pending_ticks: c.pending_ticks, spawn_areas: c.spawn_areas, ..StoredColumn::of(c.chunk) }))
    }

    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError> {
        let key = ChunkKey::new(pos.x, pos.z, dimension);
        self.0.store_column(key, &column.chunk, &column.block_entities, &column.actors, &column.pending_ticks)?;
        Ok(self.0.store_spawn_areas(key, &column.spawn_areas)?)
    }

    fn load_player(&self, id: &str) -> Result<Option<PlayerData>, StorageError> {
        Ok(self.0.load_player(id)?)
    }

    fn store_player(&self, id: &str, player: &PlayerData) -> Result<(), StorageError> {
        Ok(self.0.store_player(id, player)?)
    }

    fn columns(&self, dimension: i32) -> Result<Vec<ChunkPos>, StorageError> {
        Ok(self.0.columns()?.into_iter().filter(|key| key.dimension == dimension).map(|key| ChunkPos::new(key.x, key.z)).collect())
    }

    fn flush(&self, level: &Level) -> Result<(), StorageError> {
        let mut dat = self.0.level().clone();
        dat.set_last_played(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64);
        // The game's own stamp of this version carries a revision ours lacks: keep it.
        let version = game_version();
        if !dat.last_opened_with().is_some_and(|stamp| stamp.get(..3) == Some(&version[..3])) {
            dat.set_last_opened_with(version);
        }
        dat.set_time(level.time);
        dat.set_age(level.age);
        dat.set_weather(level.weather);
        if let Some(difficulty) = level.difficulty {
            dat.set_difficulty(difficulty);
        }
        if let Some(chunks) = level.chunk_tick_range {
            dat.set_chunk_tick_range(chunks);
        }
        write_rules(&mut dat, &level.rules);
        if let Some(spawn) = level.spawn {
            dat.set_spawn(spawn);
        }
        self.0.store_portals(&level.portals)?;
        if let Some(board) = &level.scoreboard {
            self.0.store_scoreboard(board)?;
        }
        if let Some(fight) = &level.dragon_fight {
            self.0.store_dragon_fight(fight)?;
        }
        if let Some(switches) = &level.mob_events {
            sapling_leveldb::store_mob_events(self.0.db(), switches)?;
        }
        let structures: Vec<(String, Vec<u8>)> = level.structures.iter().map(|(name, bytes)| (name.clone(), bytes.clone())).collect();
        self.0.store_structures(&structures)?;
        self.0.store_ticking_areas(&level.ticking_areas)?;
        self.0.store_maps(&level.maps)?;
        self.0.store_trackers(&level.trackers, level.last_tracker)?;
        self.0.store_villages(&level.villages)?;
        Ok(self.0.flush(&dat)?)
    }

    fn sync(&self) -> Result<(), StorageError> {
        Ok(self.0.db().flush()?)
    }

    fn folder(&self) -> Option<std::path::PathBuf> {
        Some(self.0.folder().to_path_buf())
    }
}
