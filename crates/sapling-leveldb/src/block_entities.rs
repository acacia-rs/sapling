//! The BlockEntities record (records.rs): each compound names its block in `x`, `y`, `z`.

use acacia_proto::nbt::{self, LittleEndian, Nbt, Network, Value};
use bytes::{Bytes, BytesMut};

use crate::Error;
use crate::container::without_items;
use crate::records::compounds;

/// What a block holds beyond its state: a sign's text, a chest's items.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockEntity {
    pub pos: [i32; 3],
    pub nbt: Value,
}

impl BlockEntity {
    /// What clients are sent of it, in a chunk or a `BlockEntityData` packet.
    pub fn shown(&self) -> Value {
        without_items(&self.nbt)
    }

    /// [`BlockEntity::shown`] as a chunk carries it.
    pub fn network(&self) -> Bytes {
        let mut out = BytesMut::new();
        nbt::write::<Network>(&mut out, &Nbt { name: String::new(), value: self.shown() });
        out.freeze()
    }
}

/// The record with `entity` as the one block entity of block `pos`.
pub fn with_block_entity(record: &[u8], pos: [i32; 3], entity: &Value) -> Result<Vec<u8>, Error> {
    let mut out = BytesMut::from(&without_block_entity(record, pos)?[..]);
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: entity.clone() });
    Ok(out.to_vec())
}

fn position(entity: &Value) -> [Option<i32>; 3] {
    ["x", "y", "z"].map(|key| match entity.get(key) {
        Some(Value::Int(at)) => Some(*at),
        _ => None,
    })
}

/// The block entities of a record; compounds that name no block are left out.
pub fn block_entities(record: &[u8]) -> Result<Vec<BlockEntity>, Error> {
    let placed = |(_, nbt): (&[u8], Value)| match position(&nbt) {
        [Some(x), Some(y), Some(z)] => Some(BlockEntity { pos: [x, y, z], nbt }),
        _ => None,
    };
    Ok(compounds(record)?.into_iter().filter_map(placed).collect())
}

/// The record without the compounds of block `pos`; the others keep their bytes.
pub fn without_block_entity(record: &[u8], pos: [i32; 3]) -> Result<Vec<u8>, Error> {
    let elsewhere = |entity: &Value| position(entity) != pos.map(Some);
    Ok(compounds(record)?.into_iter().filter(|(_, entity)| elsewhere(entity)).flat_map(|(raw, _)| raw.iter().copied()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::compound;

    fn entity(id: &str, [x, y, z]: [i32; 3]) -> Vec<u8> {
        compound(vec![("id", Value::String(id.into())), ("x", Value::Int(x)), ("y", Value::Int(y)), ("z", Value::Int(z))])
    }

    #[test]
    fn only_the_compounds_of_the_block_leave() {
        let (chest, sign) = (entity("Chest", [1, 64, -3]), entity("Sign", [1, 65, -3]));
        let record = [&chest[..], &sign, &chest].concat();
        assert_eq!(without_block_entity(&record, [1, 64, -3]).unwrap(), sign);
        assert_eq!(without_block_entity(&record, [1, 65, -3]).unwrap(), [&chest[..], &chest].concat());
        assert_eq!(without_block_entity(&record, [0, 0, 0]).unwrap(), record);
        assert_eq!(without_block_entity(&[], [0, 0, 0]).unwrap(), Vec::<u8>::new());
        assert!(without_block_entity(&record[..record.len() - 2], [0, 0, 0]).is_err());
    }

    #[test]
    fn a_record_reads_as_block_entities_a_client_can_read() {
        let nameless = compound(vec![("id", Value::String("Chest".into()))]);
        let record = [&entity("Sign", [1, 65, -3])[..], &nameless, &entity("Chest", [2, 4, 0])].concat();
        let read = block_entities(&record).unwrap();
        assert_eq!(read.iter().map(|entity| entity.pos).collect::<Vec<_>>(), [[1, 65, -3], [2, 4, 0]]);
        let wire = read[0].network();
        let back = nbt::read::<Network>(&mut &wire[..]).unwrap();
        assert_eq!(back.value, read[0].nbt);
        assert_eq!(back.value.get("id"), Some(&Value::String("Sign".into())));
    }
}
