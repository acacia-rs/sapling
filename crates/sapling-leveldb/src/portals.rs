//! The world's portal records, under the key `portals`: a compound `data` holding the list
//! `PortalRecords`, one compound per lit portal of any dimension: `DimId`, the inside's first
//! lowest cell `TpX`/`TpY`/`TpZ`, its width `Span` and `Xa`/`Za` for the axis it runs along
//! (a BDS 1.26.5x save, research/captures/bds-portal-far.txt). BDS finds portals by these.

use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Value};
use bytes::BytesMut;

use crate::Error;

pub(crate) const KEY: &[u8] = b"portals";
const COMPOUND: u8 = 10;

/// One portal as BDS keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortalRecord {
    pub dimension: i32,
    /// The inside's lowest cell at its start along the axis.
    pub corner: [i32; 3],
    pub span: u8,
    /// The inside runs along x (`Xa` 1), else along z (`Za` 1).
    pub along_x: bool,
}

pub fn decode_portals(record: &[u8]) -> Result<Vec<PortalRecord>, Error> {
    let root = nbt::read::<LittleEndian>(&mut &record[..])?.value;
    let Some(Value::List(list)) = root.get("data").and_then(|data| data.get("PortalRecords")) else { return Ok(Vec::new()) };
    let int = |kept: &Value, key: &str| match kept.get(key) {
        Some(Value::Int(value)) => Some(*value),
        Some(Value::Byte(value)) => Some(i32::from(*value)),
        _ => None,
    };
    let portal = |kept: &Value| {
        let corner = [int(kept, "TpX")?, int(kept, "TpY")?, int(kept, "TpZ")?];
        Some(PortalRecord { dimension: int(kept, "DimId")?, corner, span: int(kept, "Span")? as u8, along_x: int(kept, "Xa")? != 0 })
    };
    Ok(list.items.iter().filter_map(portal).collect())
}

pub fn encode_portals(portals: &[PortalRecord]) -> Vec<u8> {
    let portal = |portal: &PortalRecord| {
        let [x, y, z] = portal.corner;
        Value::Compound(vec![
            ("DimId".into(), Value::Int(portal.dimension)),
            ("Span".into(), Value::Byte(portal.span as i8)),
            ("TpX".into(), Value::Int(x)),
            ("TpY".into(), Value::Int(y)),
            ("TpZ".into(), Value::Int(z)),
            ("Xa".into(), Value::Byte(i8::from(portal.along_x))),
            ("Za".into(), Value::Byte(i8::from(!portal.along_x))),
        ])
    };
    let records = Value::List(List { tag: COMPOUND, items: portals.iter().map(portal).collect() });
    let root = Value::Compound(vec![("data".into(), Value::Compound(vec![("PortalRecords".into(), records)]))]);
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: root });
    out.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portals_read_back_as_they_were_written() {
        let portals = [PortalRecord { dimension: 1, corner: [-15, 34, 7], span: 2, along_x: false }, PortalRecord { dimension: 0, corner: [164, -59, 5], span: 21, along_x: true }];
        assert_eq!(decode_portals(&encode_portals(&portals)).unwrap(), portals);
        assert!(decode_portals(&encode_portals(&[])).unwrap().is_empty());
        assert!(decode_portals(&[1, 2]).is_err());
    }
}
