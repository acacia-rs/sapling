//! The BlockEntities record (records.rs): each compound names its block in `x`, `y`, `z`.

use acacia_proto::nbt::Value;

use crate::Error;
use crate::records::compounds;

/// The record without the compounds of block `pos`; the others keep their bytes.
pub fn without_block_entity(record: &[u8], pos: [i32; 3]) -> Result<Vec<u8>, Error> {
    let coordinate = |entity: &Value, key: &str| match entity.get(key) {
        Some(Value::Int(at)) => Some(*at),
        _ => None,
    };
    let elsewhere = |entity: &Value| ["x", "y", "z"].map(|key| coordinate(entity, key)) != pos.map(Some);
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
}
