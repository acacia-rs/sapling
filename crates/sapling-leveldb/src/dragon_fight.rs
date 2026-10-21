//! The End's dragon fight, under the key `TheEnd`: a compound `data` holding `DragonFight` and
//! `LimboEntities`, as a BDS 1.26.5x save holds it (research/captures/bds-dragon-*.txt, the
//! drill world's log). `Gateways` lists the gateways a kill has yet to open; the last goes first.

use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Value};
use bytes::BytesMut;

use crate::Error;

pub(crate) const KEY: &[u8] = b"TheEnd";
const INT: u8 = 3;
const END: u8 = 0;

/// One End's fight as BDS keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DragonFightRecord {
    pub killed: bool,
    pub previously_killed: bool,
    pub spawned: bool,
    /// The dragon's `UniqueID`; -1 for none.
    pub dragon: i64,
    /// The exit portal's middle cell, once it was placed.
    pub exit_portal: Option<[i32; 3]>,
    pub gateways: Vec<i32>,
    pub respawning: bool,
}

fn flag(fight: &Value, key: &str) -> bool {
    matches!(fight.get(key), Some(Value::Byte(value)) if *value != 0)
}

fn ints(fight: &Value, key: &str) -> Vec<i32> {
    match fight.get(key) {
        Some(Value::List(list)) => list.items.iter().filter_map(|item| if let Value::Int(value) = item { Some(*value) } else { None }).collect(),
        _ => Vec::new(),
    }
}

/// `None` for a record without a fight in it.
pub fn decode_dragon_fight(record: &[u8]) -> Result<Option<DragonFightRecord>, Error> {
    let root = nbt::read::<LittleEndian>(&mut &record[..])?.value;
    let Some(fight) = root.get("data").and_then(|data| data.get("DragonFight")) else { return Ok(None) };
    let dragon = match fight.get("DragonUUID") {
        Some(Value::Long(id)) => *id,
        _ => -1,
    };
    let portal = ints(fight, "ExitPortalLocation");
    Ok(Some(DragonFightRecord {
        killed: flag(fight, "DragonKilled"),
        previously_killed: flag(fight, "PreviouslyKilled"),
        spawned: flag(fight, "DragonSpawned"),
        dragon,
        exit_portal: <[i32; 3]>::try_from(portal).ok(),
        gateways: ints(fight, "Gateways"),
        respawning: flag(fight, "IsRespawning"),
    }))
}

pub fn encode_dragon_fight(fight: &DragonFightRecord) -> Vec<u8> {
    let list = |values: &[i32]| Value::List(List { tag: INT, items: values.iter().map(|value| Value::Int(*value)).collect() });
    let byte = |on: bool| Value::Byte(i8::from(on));
    let mut kept = vec![("DragonFightVersion".into(), Value::Byte(0)), ("DragonKilled".into(), byte(fight.killed)), ("DragonSpawned".into(), byte(fight.spawned)), ("DragonUUID".into(), Value::Long(fight.dragon))];
    if let Some(portal) = fight.exit_portal {
        kept.push(("ExitPortalLocation".into(), list(&portal)));
    }
    kept.extend([("Gateways".into(), list(&fight.gateways)), ("IsRespawning".into(), byte(fight.respawning)), ("PreviouslyKilled".into(), byte(fight.previously_killed))]);
    let limbo = Value::List(List { tag: END, items: Vec::new() });
    let data = Value::Compound(vec![("DragonFight".into(), Value::Compound(kept)), ("LimboEntities".into(), limbo)]);
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(vec![("data".into(), data)]) });
    out.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The drill world's record mid-fight, as BDS wrote it (seed 20261018).
    const BDS: &[u8] = b"\n\x00\x00\n\x04\x00data\n\x0b\x00DragonFight\x01\x12\x00DragonFightVersion\x00\x01\x0c\x00DragonKilled\x00\x01\r\x00DragonSpawned\x01\x04\n\x00DragonUUID\x08\x00\x00\x00\xff\xff\xff\xff\t\x12\x00ExitPortalLocation\x03\x03\x00\x00\x00\x00\x00\x00\x00>\x00\x00\x00\x00\x00\x00\x00\t\x08\x00Gateways\x03\x14\x00\x00\x00\x0c\x00\x00\x00\n\x00\x00\x00\x03\x00\x00\x00\x01\x00\x00\x00\x02\x00\x00\x00\x06\x00\x00\x00\x13\x00\x00\x00\r\x00\x00\x00\x0b\x00\x00\x00\x10\x00\x00\x00\t\x00\x00\x00\x05\x00\x00\x00\x0e\x00\x00\x00\x12\x00\x00\x00\x04\x00\x00\x00\x07\x00\x00\x00\x08\x00\x00\x00\x00\x00\x00\x00\x11\x00\x00\x00\x0f\x00\x00\x00\x01\x0c\x00IsRespawning\x00\x01\x10\x00PreviouslyKilled\x00\x00\t\r\x00LimboEntities\x00\x00\x00\x00\x00\x00\x00";

    #[test]
    fn bdss_record_reads_and_is_written_back_byte_for_byte() {
        let fight = decode_dragon_fight(BDS).unwrap().unwrap();
        assert_eq!((fight.killed, fight.spawned, fight.previously_killed, fight.respawning), (false, true, false, false));
        assert_eq!((fight.dragon, fight.exit_portal), (-4294967288, Some([0, 62, 0])));
        assert_eq!(fight.gateways, [12, 10, 3, 1, 2, 6, 19, 13, 11, 16, 9, 5, 14, 18, 4, 7, 8, 0, 17, 15]);
        assert_eq!(encode_dragon_fight(&fight), BDS);
        assert_eq!(decode_dragon_fight(&encode_dragon_fight(&DragonFightRecord { dragon: -1, ..Default::default() })).unwrap().unwrap().exit_portal, None);
    }
}
