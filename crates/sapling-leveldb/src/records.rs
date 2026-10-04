//! Records that are little-endian NBT compounds one after another: block entities and actors.

use acacia_proto::nbt::{self, LittleEndian, Value};

use crate::Error;

/// Each compound of `record` with the bytes it was read from.
pub(crate) fn compounds(record: &[u8]) -> Result<Vec<(&[u8], Value)>, Error> {
    let mut out = Vec::new();
    let mut rest = record;
    while !rest.is_empty() {
        let start = rest;
        let value = nbt::read::<LittleEndian>(&mut rest)?.value;
        out.push((&start[..start.len() - rest.len()], value));
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) fn compound(fields: Vec<(&str, Value)>) -> Vec<u8> {
    let mut out = bytes::BytesMut::new();
    let value = Value::Compound(fields.into_iter().map(|(key, value)| (key.to_string(), value)).collect());
    nbt::write::<LittleEndian>(&mut out, &nbt::Nbt { name: String::new(), value });
    out.to_vec()
}
