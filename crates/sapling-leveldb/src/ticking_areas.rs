//! Ticking areas, each under its own key `tickingarea_<uuid>`: a compound of `Dimension`, `Name`,
//! `IsCircle`, the block bounds `MinX`, `MinZ`, `MaxX`, `MaxZ` and `Preload`, as BDS 1.26.52 reads
//! them (its loader checks those types and shifts the bounds to chunks; an `EntityId` long marks an
//! entity's own area, which sapling does not make).

use acacia_proto::nbt::{self, LittleEndian, Nbt, Value};
use bytes::BytesMut;

use crate::Error;

pub const TICKING_AREA_PREFIX: &[u8] = b"tickingarea_";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickingAreaRecord {
    /// The text after the key's prefix.
    pub id: String,
    pub dimension: i32,
    pub name: String,
    pub circle: bool,
    /// Block x and z of the lowest corner, then of the highest.
    pub min: [i32; 2],
    pub max: [i32; 2],
    pub preload: bool,
}

impl TickingAreaRecord {
    pub fn key(&self) -> Vec<u8> {
        [TICKING_AREA_PREFIX, self.id.as_bytes()].concat()
    }

    pub fn encode(&self) -> Vec<u8> {
        let root = Value::Compound(vec![
            ("Dimension".into(), Value::Int(self.dimension)),
            ("IsCircle".into(), Value::Byte(i8::from(self.circle))),
            ("MaxX".into(), Value::Int(self.max[0])),
            ("MaxZ".into(), Value::Int(self.max[1])),
            ("MinX".into(), Value::Int(self.min[0])),
            ("MinZ".into(), Value::Int(self.min[1])),
            ("Name".into(), Value::String(self.name.as_str().into())),
            ("Preload".into(), Value::Byte(i8::from(self.preload))),
        ]);
        let mut out = BytesMut::new();
        nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: root });
        out.to_vec()
    }

    /// The area kept under `key`; `None` for another key and for an entity's area.
    pub fn decode(key: &[u8], record: &[u8]) -> Result<Option<Self>, Error> {
        let Some(id) = key.strip_prefix(TICKING_AREA_PREFIX).and_then(|id| std::str::from_utf8(id).ok()) else { return Ok(None) };
        let root = nbt::read::<LittleEndian>(&mut &record[..])?.value;
        if root.get("EntityId").is_some() {
            return Ok(None);
        }
        let int = |key: &str| match root.get(key) {
            Some(Value::Int(value)) => Ok(*value),
            _ => Err(Error::TickingArea("a bound or dimension that is no int")),
        };
        let byte = |key: &str| matches!(root.get(key), Some(Value::Byte(on)) if *on != 0);
        let name = match root.get("Name") {
            Some(Value::String(name)) => name.to_string(),
            _ => return Err(Error::TickingArea("no name")),
        };
        let (min, max) = ([int("MinX")?, int("MinZ")?], [int("MaxX")?, int("MaxZ")?]);
        Ok(Some(Self { id: id.to_owned(), dimension: int("Dimension")?, name, circle: byte("IsCircle"), min, max, preload: byte("Preload") }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_area_reads_back_as_it_was_written() {
        let area = TickingAreaRecord { id: "0b6e8c33-6c1e-4d4a-9f0e-6a5b2f7f4c11".into(), dimension: 1, name: "farm".into(), circle: true, min: [-32, 16], max: [31, 79], preload: true };
        assert_eq!(TickingAreaRecord::decode(&area.key(), &area.encode()).unwrap(), Some(area.clone()));
        assert_eq!(TickingAreaRecord::decode(b"portals", &area.encode()).unwrap(), None);
        assert!(TickingAreaRecord::decode(&area.key(), &[10, 0, 0, 0]).is_err());
    }
}
