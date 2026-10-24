//! A world folder: `level.dat` beside the `db` directory.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use acacia_world::{BlockRegistry, Chunk, Dimension, SECTION_VOLUME};

use crate::data3d::Data3d;
use crate::keys::{ChunkKey, Tag};
use crate::states::BlockStates;
use crate::sub_chunk::SubChunk;
use crate::{Batch, Db, Error, LevelDat, PlayerData, actors, player};

/// The chunk format version written; BDS 1.26.52 saves carry the same (tests/world.rs).
const CHUNK_VERSION: u8 = 42;
/// `FinalizedState` of a column the game need not generate or populate.
const FINALIZED: u32 = 2;

/// One chunk column as saved.
#[derive(Debug, Clone, PartialEq)]
pub struct DiskColumn {
    pub chunk: Chunk,
    /// The chunk format version byte (41/42 on 1.21+ worlds).
    pub version: u8,
    /// Heightmap and biomes; the biomes are already in `chunk`.
    pub data3d: Option<Data3d>,
    /// Concatenated little-endian NBT compounds, kept as stored (block_entities.rs).
    pub block_entities: Vec<u8>,
    /// The column's actors as one record of compounds (actors.rs).
    pub actors: Vec<u8>,
}

pub struct LevelDbWorld {
    folder: PathBuf,
    level: LevelDat,
    db: Db,
    in_memory: bool,
    states: BlockStates,
}

impl LevelDbWorld {
    /// Opens a world folder for reading and writing (see [`Db::open`]).
    pub fn open(folder: &Path) -> Result<Self, Error> {
        Self::with_db(folder, Db::open(&folder.join("db"))?, false)
    }

    /// Opens a copy of the world held in memory, leaving the folder untouched
    /// (see [`Db::open_in_memory`]).
    pub fn open_in_memory(folder: &Path) -> Result<Self, Error> {
        Self::with_db(folder, Db::open_in_memory(&folder.join("db"))?, true)
    }

    fn with_db(folder: &Path, db: Db, in_memory: bool) -> Result<Self, Error> {
        let level = LevelDat::read(&folder.join("level.dat"))?;
        Ok(Self { folder: folder.to_path_buf(), level, db, in_memory, states: BlockStates::new(BlockRegistry::vanilla_arc()) })
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    pub fn level(&self) -> &LevelDat {
        &self.level
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn registry(&self) -> &Arc<BlockRegistry> {
        self.states.registry()
    }

    /// Reads a column; `None` when it was never saved. A section that does not decode is logged
    /// and left as air, so one damaged record does not cost the column.
    pub fn load_column(&self, key: ChunkKey) -> Result<Option<DiskColumn>, Error> {
        let version = match self.db.get(&key.tag(Tag::Version))? {
            Some(version) => version,
            None => match self.db.get(&key.tag(Tag::LegacyVersion))? {
                Some(version) => version,
                None => return Ok(None),
            },
        };
        let dim = Dimension::from_id(key.dimension, self.registry().air_id());
        let mut chunk = Chunk::empty(key.x, key.z, dim);
        let lowest = dim.min_y >> 4;
        for y in lowest..lowest + chunk.section_count() as i32 {
            let Some(record) = self.db.get(&key.sub_chunk(y as i8))? else { continue };
            match SubChunk::decode(&record, &self.states) {
                Ok(section) => {
                    if let Some(blocks) = section.layers.first() {
                        chunk.set_section(y, blocks, section.layers.get(1).map(|liquid| &**liquid));
                    }
                }
                Err(error) => tracing::warn!(x = key.x, z = key.z, y, %error, "skipping an unreadable sub-chunk"),
            }
        }
        let data3d = match self.db.get(&key.tag(Tag::Data3d))?.map(|record| Data3d::decode(&record)) {
            Some(Ok(data)) => Some(data),
            Some(Err(error)) => {
                tracing::warn!(x = key.x, z = key.z, %error, "skipping unreadable biomes");
                None
            }
            None => None,
        };
        if let Some(data) = &data3d {
            chunk.set_biomes(&data.biomes);
        }
        let block_entities = self.db.get(&key.tag(Tag::BlockEntities))?.unwrap_or_default();
        let actors = actors::load(&self.db, key)?;
        Ok(Some(DiskColumn { chunk, version: version.first().copied().unwrap_or(0), data3d, block_entities, actors }))
    }

    /// Writes a column's blocks, biomes, heightmap, block entities and actors as one batch. Its
    /// other records (pending ticks) stay as they are.
    pub fn store_column(&self, key: ChunkKey, chunk: &Chunk, block_entities: &[u8], actors: &[u8]) -> Result<(), Error> {
        let mut batch = Batch::default();
        actors::store(&self.db, &mut batch, key, actors)?;
        match block_entities {
            [] => batch.delete(&key.tag(Tag::BlockEntities)),
            record => batch.put(&key.tag(Tag::BlockEntities), record),
        }
        batch.put(&key.tag(Tag::Version), &[CHUNK_VERSION]);
        batch.delete(&key.tag(Tag::LegacyVersion));
        batch.put(&key.tag(Tag::FinalizedState), &FINALIZED.to_le_bytes());
        batch.put(&key.tag(Tag::Data3d), &Data3d::of(chunk, self.registry()).encode()?);
        let air = self.registry().air_id();
        let lowest = chunk.dimension().min_y >> 4;
        let (mut blocks, mut liquid) = (Box::new([air; SECTION_VOLUME]), Box::new([air; SECTION_VOLUME]));
        for index in 0..chunk.section_count() {
            let y = (lowest + index as i32) as i8;
            // An absent section leaves the buffers holding the one below.
            let present = chunk.copy_section(index, &mut blocks, &mut liquid);
            let wet = present && liquid.iter().any(|&id| id != air);
            if !present || (!wet && blocks.iter().all(|&id| id == air)) {
                batch.delete(&key.sub_chunk(y));
                continue;
            }
            let layers = if wet { vec![blocks.clone(), liquid.clone()] } else { vec![blocks.clone()] };
            batch.put(&key.sub_chunk(y), &SubChunk { y: Some(y), layers }.encode(&self.states)?);
        }
        self.db.write(batch)
    }

    /// What was kept of the player with this id (its UUID as text); `None` for a first visit.
    pub fn load_player(&self, id: &str) -> Result<Option<PlayerData>, Error> {
        player::load(&self.db, id)
    }

    pub fn store_player(&self, id: &str, player: &PlayerData) -> Result<(), Error> {
        player::store(&self.db, id, player, self.registry())
    }

    /// The portal records of every dimension (portals.rs); none in a world without the key.
    pub fn load_portals(&self) -> Result<Vec<crate::PortalRecord>, Error> {
        self.db.get(crate::portals::KEY)?.map_or(Ok(Vec::new()), |record| crate::decode_portals(&record))
    }

    pub fn store_portals(&self, portals: &[crate::PortalRecord]) -> Result<(), Error> {
        self.db.put(crate::portals::KEY, &crate::encode_portals(portals))
    }

    /// The End's dragon fight (dragon_fight.rs); `None` before one began.
    pub fn load_dragon_fight(&self) -> Result<Option<crate::DragonFightRecord>, Error> {
        self.db.get(crate::dragon_fight::KEY)?.map_or(Ok(None), |record| crate::decode_dragon_fight(&record))
    }

    pub fn store_dragon_fight(&self, fight: &crate::DragonFightRecord) -> Result<(), Error> {
        self.db.put(crate::dragon_fight::KEY, &crate::encode_dragon_fight(fight))
    }

    /// Writes what was stored out as a table file and replaces `level.dat` with `level`. A world
    /// opened in memory flushes its copy and leaves the folder alone.
    pub fn flush(&self, level: &LevelDat) -> Result<(), Error> {
        self.db.flush()?;
        if self.in_memory {
            return Ok(());
        }
        level.write(&self.folder.join("level.dat"))
    }

    /// Every saved column, in key order.
    pub fn columns(&self) -> Result<Vec<ChunkKey>, Error> {
        let mut columns = Vec::new();
        self.db.for_each(|key, _| {
            if let Some(parsed) = ChunkKey::parse(key).filter(|k| k.tag == Tag::Version as u8 || k.tag == Tag::LegacyVersion as u8) {
                columns.push(parsed.chunk);
            }
        })?;
        Ok(columns)
    }
}
