//! `level.dat`: `i32 storage version | i32 length | little-endian NBT`.

use std::path::Path;

use acacia_proto::nbt::{self, LittleEndian, Nbt, Value};
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

    pub fn seed(&self) -> Option<i64> {
        match self.root.get("RandomSeed")? {
            Value::Long(v) => Some(*v),
            _ => None,
        }
    }

    /// The world spawn as saved. Worlds whose spawn was never set by hand keep y 32767 ("the
    /// surface"): callers pick the y from the terrain.
    pub fn spawn(&self) -> Option<[i32; 3]> {
        Some([self.int("SpawnX")?, self.int("SpawnY")?, self.int("SpawnZ")?])
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
