//! `level.dat`: `i32 storage version | i32 length | little-endian NBT`.

use std::path::Path;

use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Value};
use bytes::BytesMut;

use crate::Error;

const HEADER: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct LevelDat {
    /// 10 since 1.16.
    pub storage_version: i32,
    /// The root compound; fields this type has no accessor for are read from here.
    pub root: Value,
}

impl LevelDat {
    pub fn read(path: &Path) -> Result<Self, Error> {
        Self::parse(&std::fs::read(path).map_err(|e| Error::Io(path.to_path_buf(), e))?)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let int = |at: usize| bytes.get(at..at + 4).map(|b| i32::from_le_bytes(b.try_into().expect("four bytes")));
        let (Some(storage_version), Some(length)) = (int(0), int(4)) else { return Err(Error::LevelDat("shorter than its header")) };
        let body = usize::try_from(length).ok().and_then(|n| bytes.get(HEADER..HEADER + n)).ok_or(Error::LevelDat("shorter than its length field"))?;
        let root = nbt::read::<LittleEndian>(&mut &body[..])?.value;
        match root {
            Value::Compound(_) => Ok(Self { storage_version, root }),
            _ => Err(Error::LevelDat("the root is not a compound")),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut body = BytesMut::new();
        nbt::write::<LittleEndian>(&mut body, &Nbt { name: String::new(), value: self.root.clone() });
        [&self.storage_version.to_le_bytes()[..], &(body.len() as i32).to_le_bytes(), &body].concat()
    }

    /// Replaces the file, keeping the one it replaces as `level.dat_old` the way the game does.
    pub fn write(&self, path: &Path) -> Result<(), Error> {
        let io = |error| Error::Io(path.to_path_buf(), error);
        let new = path.with_extension("dat_new");
        std::fs::write(&new, self.to_bytes()).map_err(io)?;
        if path.exists() {
            std::fs::copy(path, path.with_extension("dat_old")).map_err(io)?;
        }
        std::fs::rename(&new, path).map_err(io)
    }

    /// Sets a field of the root, in place when it is there already.
    pub fn set(&mut self, key: &str, new: Value) {
        let Value::Compound(fields) = &mut self.root else { return };
        match fields.iter_mut().find(|(name, _)| name == key) {
            Some((_, value)) => *value = new,
            None => fields.push((key.into(), new)),
        }
    }

    /// When the world was last closed, in unix seconds.
    pub fn set_last_played(&mut self, seconds: i64) {
        self.set("LastPlayed", Value::Long(seconds));
    }

    /// The game version that last wrote the world, e.g. `[1, 26, 52, 0, 0]`.
    pub fn last_opened_with(&self) -> Option<Vec<i32>> {
        let Value::List(list) = self.root.get("lastOpenedWithVersion")? else { return None };
        list.items.iter().map(|part| if let Value::Int(part) = part { Some(*part) } else { None }).collect()
    }

    pub fn set_last_opened_with(&mut self, version: [i32; 5]) {
        self.set("lastOpenedWithVersion", Value::List(List { tag: Value::Int(0).tag(), items: version.map(Value::Int).into() }));
    }

    pub fn name(&self) -> Option<&str> {
        match self.root.get("LevelName")? {
            Value::String(name) => Some(name),
            _ => None,
        }
    }

    pub fn int(&self, key: &str) -> Option<i32> {
        match self.root.get(key)? {
            Value::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn long(&self, key: &str) -> Option<i64> {
        match self.root.get(key)? {
            Value::Long(v) => Some(*v),
            _ => None,
        }
    }

    pub fn seed(&self) -> Option<i64> {
        self.long("RandomSeed")
    }

    /// The world spawn as saved. Worlds whose spawn was never set by hand keep y 32767 ("the
    /// surface"): callers pick the y from the terrain.
    pub fn spawn(&self) -> Option<[i32; 3]> {
        Some([self.int("SpawnX")?, self.int("SpawnY")?, self.int("SpawnZ")?])
    }

    pub fn set_spawn(&mut self, [x, y, z]: [i32; 3]) {
        [("SpawnX", x), ("SpawnY", y), ("SpawnZ", z)].into_iter().for_each(|(key, at)| self.set(key, Value::Int(at)));
    }

    /// The time of day counter: it runs on past 24000, a day's length.
    pub fn time(&self) -> Option<i64> {
        self.long("Time")
    }

    pub fn set_time(&mut self, time: i64) {
        self.set("Time", Value::Long(time));
    }

    /// Whether the time of day advances (the `dodaylightcycle` game rule; on when absent).
    pub fn daylight_cycle(&self) -> bool {
        !matches!(self.root.get("dodaylightcycle"), Some(Value::Byte(0)))
    }

    /// The default game mode: 0 survival, 1 creative, 2 adventure, 6 spectator.
    pub fn game_type(&self) -> Option<i32> {
        self.int("GameType")
    }

    /// 0 peaceful to 3 hard.
    pub fn difficulty(&self) -> Option<i32> {
        self.int("Difficulty")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> LevelDat {
        let fields = [
            ("LevelName".to_string(), Value::String("lobby".into())),
            ("RandomSeed".to_string(), Value::Long(-77)),
            ("SpawnX".to_string(), Value::Int(12)),
            ("SpawnY".to_string(), Value::Int(32767)),
            ("SpawnZ".to_string(), Value::Int(-4)),
        ];
        LevelDat { storage_version: 10, root: Value::Compound(fields.into()) }
    }

    #[test]
    fn fields_survive_a_round_trip() {
        let bytes = sample().to_bytes();
        assert_eq!(bytes[..4], [10, 0, 0, 0]);
        assert_eq!(i32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize, bytes.len() - 8);
        let parsed = LevelDat::parse(&bytes).unwrap();
        assert_eq!(parsed, sample());
        assert_eq!((parsed.name(), parsed.seed(), parsed.spawn()), (Some("lobby"), Some(-77), Some([12, 32767, -4])));
        assert_eq!(parsed.int("LevelName"), None);
    }

    #[test]
    fn damaged_files_are_errors() {
        let bytes = sample().to_bytes();
        assert!(matches!(LevelDat::parse(&bytes[..6]), Err(Error::LevelDat(_))));
        assert!(matches!(LevelDat::parse(&bytes[..bytes.len() - 1]), Err(Error::LevelDat(_))));
        let mut wrong_root = bytes.clone();
        wrong_root[8] = 99;
        assert!(LevelDat::parse(&wrong_root).is_err());
    }
}
