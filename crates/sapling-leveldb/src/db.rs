//! The key-value store under a world: LevelDB with Mojang's block compressions.

use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

use flate2::Compression;
use flate2::read::{DeflateDecoder, ZlibDecoder};
use flate2::write::{DeflateEncoder, ZlibEncoder};
use rusty_leveldb::compressor::NoneCompressor;
use rusty_leveldb::env::Env;
use rusty_leveldb::{Compressor, CompressorList, DB, LdbIterator, MemEnv, Options, Status, StatusCode, WriteBatch};

use crate::Error;

/// Block compression ids in world databases (research/worlds-and-ecs.md "Mojang LevelDB variant").
const ZLIB: u8 = 2;
const RAW_DEFLATE: u8 = 4;

struct Zlib;
struct RawDeflate;

fn compression_error(error: std::io::Error) -> Status {
    Status { code: StatusCode::CompressionError, err: error.to_string() }
}

fn drain(mut reader: impl Read) -> rusty_leveldb::Result<Vec<u8>> {
    let mut out = Vec::new();
    reader.read_to_end(&mut out).map_err(compression_error)?;
    Ok(out)
}

impl Compressor for Zlib {
    fn encode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&block).map_err(compression_error)?;
        encoder.finish().map_err(compression_error)
    }

    fn decode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        drain(ZlibDecoder::new(&block[..]))
    }
}

impl Compressor for RawDeflate {
    fn encode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&block).map_err(compression_error)?;
        encoder.finish().map_err(compression_error)
    }

    fn decode(&self, block: Vec<u8>) -> rusty_leveldb::Result<Vec<u8>> {
        drain(DeflateDecoder::new(&block[..]))
    }
}

fn options(create: bool) -> Options {
    let mut compressors = CompressorList::new();
    compressors.set_with_id(0, NoneCompressor);
    compressors.set_with_id(ZLIB, Zlib);
    compressors.set_with_id(RAW_DEFLATE, RawDeflate);
    Options { create_if_missing: create, compressor: RAW_DEFLATE, compressor_list: Arc::new(compressors), ..Options::default() }
}

/// Writes applied together by [`Db::write`].
#[derive(Default)]
pub struct Batch(WriteBatch);

impl Batch {
    pub fn put(&mut self, key: &[u8], value: &[u8]) {
        self.0.put(key, value);
    }

    pub fn delete(&mut self, key: &[u8]) {
        self.0.delete(key);
    }
}

/// A world's `db` folder. Reads and writes take `&self` and may come from any thread.
pub struct Db(DB);

impl Db {
    /// Opens an existing database. As with any LevelDB, opening replays the write-ahead log and
    /// may rewrite files in the folder; [`Db::open_in_memory`] leaves the folder untouched.
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self(DB::open(path, options(false))?))
    }

    /// Opens the database at `path`, creating an empty one if there is none.
    pub fn create(path: &Path) -> Result<Self, Error> {
        Ok(Self(DB::open(path, options(true))?))
    }

    /// Reads every file of the folder into memory and opens that copy: nothing on disk changes,
    /// and writes are lost when the value is dropped. For maps that are only read (templates).
    pub fn open_in_memory(path: &Path) -> Result<Self, Error> {
        let env = MemEnv::new();
        let root = Path::new("db");
        env.mkdir(root)?;
        for entry in std::fs::read_dir(path).map_err(|e| Error::Io(path.to_path_buf(), e))? {
            let entry = entry.map_err(|e| Error::Io(path.to_path_buf(), e))?;
            // The lock belongs to whoever has the folder open; the copy takes its own.
            if entry.file_name() == "LOCK" || !entry.path().is_file() {
                continue;
            }
            let content = std::fs::read(entry.path()).map_err(|e| Error::Io(entry.path(), e))?;
            let mut file = env.open_writable_file(&root.join(entry.file_name()))?;
            file.write_all(&content).map_err(|e| Error::Io(entry.path(), e))?;
        }
        Ok(Self(DB::open(root, Options { env: Arc::new(env), ..options(false) })?))
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let mut value = Vec::new();
        Ok(self.0.get_into(key, &mut value)?.then_some(value))
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        Ok(self.0.put(key, value)?)
    }

    pub fn delete(&self, key: &[u8]) -> Result<(), Error> {
        Ok(self.0.delete(key)?)
    }

    /// Applies a batch atomically: one batch per chunk column when saving.
    pub fn write(&self, batch: Batch) -> Result<(), Error> {
        Ok(self.0.write(batch.0, false)?)
    }

    /// Writes the memtable out as a table file.
    pub fn flush(&self) -> Result<(), Error> {
        Ok(self.0.flush()?)
    }

    /// Calls `visit` with every entry in key order.
    pub fn for_each(&self, mut visit: impl FnMut(&[u8], &[u8])) -> Result<(), Error> {
        let mut entries = self.0.new_iter()?;
        while entries.advance() {
            if let Some((key, value)) = entries.current() {
                visit(&key, &value);
            }
        }
        Ok(())
    }

    /// The entries whose key starts with `prefix`, in key order, found by a seek.
    pub fn prefixed(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, Error> {
        let mut entries = self.0.new_iter()?;
        entries.seek(prefix);
        let mut found = Vec::new();
        while let Some((key, value)) = entries.current().filter(|(key, _)| key.starts_with(prefix)) {
            found.push((key, value));
            if !entries.advance() {
                break;
            }
        }
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_mojang_compressions_round_trip() {
        let block: Vec<u8> = (0..4000u32).flat_map(|i| (i % 7).to_le_bytes()).collect();
        let zlib = Zlib.encode(block.clone()).unwrap();
        let raw = RawDeflate.encode(block.clone()).unwrap();
        assert_eq!(zlib[0], 0x78, "zlib has its header");
        assert!(raw.len() < zlib.len() && zlib.len() < block.len());
        assert_eq!((Zlib.decode(zlib).unwrap(), RawDeflate.decode(raw).unwrap()), (block.clone(), block));
        assert!(RawDeflate.decode(vec![0xff; 16]).is_err());
    }
}
