//! What lodestone compasses point at, BDS's `PositionTrackingDB`: each target under
//! `PosTrackDB-0x%08x`, its handle in eight hex digits (the key is built at 14478b480), a compound
//! of `id` (the same `0x` text), `dim`, `pos` (three ints), `status` and `version` 1; the last
//! handle given out under `PositionTrackDB-LastId` as `{id}`. The packet that tells a client of one
//! carries the same compound.

use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Value};
use bytes::BytesMut;

use crate::Error;

pub const TRACKER_PREFIX: &[u8] = b"PosTrackDB-";
pub const LAST_TRACKER_KEY: &[u8] = b"PositionTrackDB-LastId";
const VERSION: i8 = 1;
const INT: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackerRecord {
    /// The `trackingHandle` of the compasses bound to it.
    pub handle: u32,
    pub dimension: i32,
    pub pos: [i32; 3],
    /// 0 while its lodestone stands; what else BDS writes was not read.
    pub status: u8,
}

fn hex(handle: u32) -> String {
    format!("0x{handle:08x}")
}

fn write(root: Value) -> Vec<u8> {
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: root });
    out.to_vec()
}

fn handle_of(root: &Value) -> Option<u32> {
    let Some(Value::String(id)) = root.get("id") else { return None };
    u32::from_str_radix(id.strip_prefix("0x")?, 16).ok()
}

impl TrackerRecord {
    pub fn key(&self) -> Vec<u8> {
        [TRACKER_PREFIX, hex(self.handle).as_bytes()].concat()
    }

    /// The record's compound, as stored and as sent.
    pub fn compound(&self) -> Value {
        let pos = Value::List(List { tag: INT, items: self.pos.map(Value::Int).into() });
        Value::Compound(vec![
            ("dim".into(), Value::Int(self.dimension)),
            ("id".into(), Value::String(hex(self.handle).as_str().into())),
            ("pos".into(), pos),
            ("status".into(), Value::Byte(self.status as i8)),
            ("version".into(), Value::Byte(VERSION)),
        ])
    }

    pub fn encode(&self) -> Vec<u8> {
        write(self.compound())
    }

    /// The target kept under `key`; `None` for another key.
    pub fn decode(key: &[u8], record: &[u8]) -> Result<Option<Self>, Error> {
        if !key.starts_with(TRACKER_PREFIX) {
            return Ok(None);
        }
        let root = nbt::read::<LittleEndian>(&mut &record[..])?.value;
        let handle = handle_of(&root).ok_or(Error::Tracker("no id"))?;
        let Some(Value::Int(dimension)) = root.get("dim") else { return Err(Error::Tracker("no dim")) };
        let pos = match root.get("pos") {
            Some(Value::List(list)) => list.items.iter().filter_map(|c| if let Value::Int(c) = c { Some(*c) } else { None }).collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let pos: [i32; 3] = pos.try_into().map_err(|_| Error::Tracker("no pos"))?;
        let status = if let Some(Value::Byte(status)) = root.get("status") { *status as u8 } else { 0 };
        Ok(Some(Self { handle, dimension: *dimension, pos, status }))
    }
}

pub fn encode_last_tracker(handle: u32) -> Vec<u8> {
    write(Value::Compound(vec![("id".into(), Value::String(hex(handle).as_str().into()))]))
}

pub fn decode_last_tracker(record: &[u8]) -> Result<u32, Error> {
    handle_of(&nbt::read::<LittleEndian>(&mut &record[..])?.value).ok_or(Error::Tracker("no last id"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_target_reads_back_as_it_was_written_under_its_hex_key() {
        let target = TrackerRecord { handle: 26, dimension: 1, pos: [-3, 70, 12], status: 0 };
        assert_eq!(target.key(), b"PosTrackDB-0x0000001a");
        assert_eq!(TrackerRecord::decode(&target.key(), &target.encode()).unwrap(), Some(target));
        assert_eq!(TrackerRecord::decode(b"portals", &target.encode()).unwrap(), None);
        assert_eq!(decode_last_tracker(&encode_last_tracker(26)).unwrap(), 26);
    }
}
